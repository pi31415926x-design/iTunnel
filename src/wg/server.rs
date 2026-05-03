//! WireGuard server-mode implementation.
//!
//! Responsibilities:
//!
//! 1. Build a server [`Wg`] by combining the on-disk peer list
//!    (`~/.itunnel/itunnel_peers.json`) with the operator-supplied identity
//!    in `.env` (`PrivateKey=...`, `InterfaceName=...` for the TUN device name,
//!    `Endpoint=host:port` or `Endpoint=[ipv6]:port` for the public UDP listen port,
//!    optional fallback `ListenPort=...` when Endpoint has no `:port`).
//! 2. Translate that into a clean uapi config (no AmneziaWG params) suitable
//!    for vanilla WireGuard peers.
//! 3. Drive the lifecycle: create a TUN device via [`crate::wg::tun`], assign
//!    its IP/MTU, and hand it to libwg-go via [`WireGuardApi::turn_on`].
//!    (NAT / full-tunnel “gateway” for operators is a client-only concern.)
//!
//! The example INI config that motivates this module lives at the top of
//! `src/wg/server.rs` (in source comments) and looks like:
//!
//! ```text
//! [Interface]
//! Address = 10.88.0.1/24
//! ListenPort = 51820
//! PrivateKey = ...
//!
//! [Peer]
//! PublicKey = ...
//! AllowedIPs = 10.88.0.2/32
//! ```
//!
//! Cross-platform notes:
//!
//! - **Windows server**: Wintun via libwg-go `createTun`; `netsh` configures IPv4 after `wgTurnOn`.

use crate::wg::config::{AmneziaParams, AppMode, ConnectionStatus, Peer, Wg, WireGuardState};
use crate::wg::tun::TunDevice;
use crate::wg::WireGuardApi;
use log::{info, warn};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::net::Ipv4Addr;
#[cfg(target_os = "macos")]
use std::process::Command;

/// Default MTU for the server TUN. 1280 is the typical conservative WG MTU
/// (matches what client mode uses). Operators can override via the INI
/// `MTU =` field once we add INI loading.
const DEFAULT_SERVER_MTU: u16 = 1280;

/// On Linux, a bare IPv4 in `Interface.Address` (e.g. `10.88.0.1`) becomes
/// `/32` in `TunDevice` address configuration, which stacks badly with
/// a `/24` VPN subnet (`10.88.0.1/24` appearing twice under one interface) and
/// can provoke TUN `write()` errors (`EDESTADDRREQ`, drops). Normalize to `/24`
/// when no prefix was given so one `ip addr add X/24 dev …` wins.
#[cfg(target_os = "linux")]
fn normalize_linux_server_tunnel_address_list(input: &str) -> String {
    input
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|part| {
            if part.contains('/') {
                return part.to_string();
            }
            if part.parse::<Ipv4Addr>().is_ok() {
                return format!("{part}/24");
            }
            part.to_string()
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(not(target_os = "linux"))]
fn normalize_linux_server_tunnel_address_list(input: &str) -> String {
    input.to_string()
}

/// Entry point: load .env + peers.json into `state.wg_config`, ensuring the
/// server's interface address / private key / listen port are populated.
/// Also reads `InterfaceName` from `.env` for the server TUN (see returned
/// `Option<String>`).
pub fn load_server_config(state: &mut WireGuardState) -> Result<(Wg, Option<String>), String> {
    debug_assert_eq!(state.app_mode, AppMode::Server);

    // Start from whatever's in memory (set by API peer add/edit), fall back
    // to disk, fall back to defaults.
    let mut wg = match &state.wg_config {
        Some(existing) => existing.clone(),
        None => Wg::default(),
    };

    if wg.peers.is_empty() {
        wg.peers = crate::wg::store::load_peers().unwrap_or_default();
    }

    let mut interface_name: Option<String> = None;

    // Pull operator identity from .env; non-fatal if missing so the WebUI
    // can still render peer management views. `InterfaceName` is always
    // parsed when the file is readable.
    //
    // WireGuard `listen_port` matches the UDP port clients use, so it is taken
    // from `Endpoint=...:port` when present; otherwise `ListenPort=`, then 51820.
    match read_env_identity() {
        Ok((pk, port_opt, endpoint_opt, ifn)) => {
            interface_name = ifn.filter(|s| !s.is_empty());
            if let Some(ep) = endpoint_opt {
                state.active_endpoint = Some(ep.clone());
                if let Some(p) = port_from_endpoint(&ep) {
                    wg.interface.listen_port = p;
                }
            }
            if !pk.is_empty() && wg.interface.private_key.is_empty() {
                wg.interface.private_key = pk;
            }
            if wg.interface.listen_port == 0 {
                if let Some(p) = port_opt {
                    wg.interface.listen_port = p;
                }
            }
        }
        Err(e) => warn!("Could not read .env identity: {}", e),
    }

    if wg.interface.listen_port == 0 {
        wg.interface.listen_port = 51820;
    }
    if wg.interface.address.is_empty() {
        // Sensible default for the example config in src/wg/server.rs.
        wg.interface.address = "10.88.0.1/24".to_string();
    }
    if wg.interface.mtu == 0 {
        wg.interface.mtu = DEFAULT_SERVER_MTU;
    }

    state.wg_config = Some(wg.clone());
    Ok((wg, interface_name))
}

/// Build the libwg-go uapi config for the server.
///
/// Note: this server path explicitly clears `h1..h4` to avoid Amnezia hash
/// obfuscation side effects while debugging baseline connectivity.
pub fn build_uapi(wg: &Wg) -> Result<String, String> {
    if wg.interface.private_key.is_empty() {
        return Err(
            "Server PrivateKey is empty. Set PrivateKey=... in the project root .env file."
                .to_string(),
        );
    }

    let priv_hex = base64_to_hex(&wg.interface.private_key)
        .map_err(|e| format!("Server PrivateKey decode failed: {}", e))?;

    let mtu = if wg.interface.mtu == 0 {
        DEFAULT_SERVER_MTU
    } else {
        wg.interface.mtu
    };
    let mut s = String::new();
    s.push_str(&format!("private_key={}\n", priv_hex));
    s.push_str(&format!("listen_port={}\n", wg.interface.listen_port));
    s.push_str("replace_peers=true\n");
    // add amnezia params
    if let Some(amnezia_params) = &wg.interface.amnezia_params {
        s.push_str(&format!("jc={}\n", amnezia_params.jc));
        s.push_str(&format!("jmin={}\n", amnezia_params.jmin));
        s.push_str(&format!("jmax={}\n", amnezia_params.jmax));
        s.push_str(&format!("s1={}\n", amnezia_params.s1));
        s.push_str(&format!("s2={}\n", amnezia_params.s2));
        s.push_str(&format!("h1={}\n", amnezia_params.h1));
        s.push_str(&format!("h2={}\n", amnezia_params.h2));
        s.push_str(&format!("h3={}\n", amnezia_params.h3));
        s.push_str(&format!("h4={}\n", amnezia_params.h4));
        s.push_str(&format!("i1={}\n", amnezia_params.i1));
    }
    
    for p in &wg.peers {
        if p.public_key.is_empty() {
            warn!("Skipping peer '{}' with empty PublicKey", p.name.clone().unwrap_or_default());
            continue;
        }
        let pub_hex = base64_to_hex(&p.public_key)
            .map_err(|e| format!("Peer PublicKey decode failed: {}", e))?;
        s.push_str(&format!("public_key={}\n", pub_hex));

        if !p.preshared_key.is_empty() {
            let psk_hex = base64_to_hex(&p.preshared_key)
                .map_err(|e| format!("Peer PresharedKey decode failed: {}", e))?;
            s.push_str(&format!("preshared_key={}\n", psk_hex));
        }
        for ip in &p.allowed_ips {
            if !ip.trim().is_empty() {
                s.push_str(&format!("allowed_ip={}\n", ip.trim()));
            }
        }
        // Never emit `endpoint=` in server mode: roaming clients negotiate address
        // from incoming UDP only; persisted peer.endpoint may still exist in UI/JSON for display/export.

        //TODO: 这里需要根据配置文件来决定是否启用持久化心跳
        // if let Some(ka) = p.persistent_keepalive {
        //     if ka > 0 {
        //         s.push_str(&format!("persistent_keepalive_interval={}\n", ka));
        //     }
        // }
    }

    Ok(s)
}

/// Start the WireGuard server.
///
/// Steps:
/// 1. Resolve the [`Wg`] config from state + disk + .env.
/// 2. Create the TUN device and assign IP/MTU.
/// 3. Hand the fd to libwg-go via `wgTurnOn`.
///
/// On error, any partially-allocated TUN is closed before returning.
pub fn start(state: &mut WireGuardState, protocol_obfuscation: bool) -> Result<(), String> {
    if state.app_mode != AppMode::Server {
        return Err("server::start called outside server mode".to_string());
    }
    if state.handle.is_some() {
        info!("Server is already running (handle={:?})", state.handle);
        return Ok(());
    }

    let (mut wg, tun_name_opt) = load_server_config(state)?;
    if protocol_obfuscation {
        wg.interface.amnezia_params = read_env_amnezia_params()?;
        state.wg_config = Some(wg.clone());
    } else {
        wg.interface.amnezia_params = None;
        state.wg_config = Some(wg.clone());
    }
    let wg_uapi = wg_config_for_device_uapi(state, &wg);
    let uapi = build_uapi(&wg_uapi)?;

    // 1. TUN (name from .env: InterfaceName=...)
    let iface = tun_name_opt
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Server TUN name is not set. Add InterfaceName=... to the project root .env (e.g. \
             utun88 on macOS, wg0 on Linux, itunnel on Windows with Wintun)."
                .to_string()
        })?;

    WireGuardApi::set_logger();
    let mtu = if wg.interface.mtu == 0 {
        DEFAULT_SERVER_MTU
    } else {
        wg.interface.mtu
    };
    let tun = TunDevice::create(&iface, mtu)
        .map_err(|e| format!("Create TUN '{}' failed: {}", iface, e))?;

    let tunnel_addrs = normalize_linux_server_tunnel_address_list(&wg.interface.address);

    #[cfg(target_os = "linux")]
    {
        // Let libwg-go call `setMTU` on a pristine tun fd first; then add addresses and `ip link`.
        let session_fd = tun.fd;
        // Hold a duplicate fd for teardown only. Never close the session fd after a successful
        // wgTurnOn — libwg-go owns it. Closing the dup after wgTurnOff releases the kernel TUN
        // if the library left a refcount, without risking close(2) on a recycled fd number.
        let cleanup_fd = dup_tun_for_teardown(session_fd)?;
        state.tun_fd = Some(cleanup_fd);
        log::debug!("uapi: {}", uapi);
        let handle = match WireGuardApi::turn_on(&uapi, session_fd) {
            Ok(h) => h,
            Err(e) => {
                close_fd_best_effort("linux wgTurnOn err (session)", session_fd);
                if let Some(c) = state.tun_fd.take() {
                    close_fd_best_effort("linux wgTurnOn err (dup)", c);
                }
                state.status = ConnectionStatus::Error;
                return Err(format!("wgTurnOn failed: {}", e));
            }
        };
        if let Err(e) = tun.configure_address(&tunnel_addrs) {
            WireGuardApi::turn_off(handle);
            if let Some(c) = state.tun_fd.take() {
                close_fd_best_effort("linux configure_addr (dup)", c);
            }
            state.status = ConnectionStatus::Error;
            return Err(format!(
                "Configure {} on {} failed: {}",
                tunnel_addrs, iface, e
            ));
        }
        if let Err(e) = tun.linux_apply_mtu_up(mtu) {
            warn!(
                "Post-wgTurnOn ip link mtu/up on {} failed (tunnel may use default link state): {}",
                iface, e
            );
        }
        if let Err(e) = ensure_server_routes(&iface, &tunnel_addrs) {
            warn!("Failed to ensure server routes on {}: {}", iface, e);
        }
        state.handle = Some(handle);
        state.status = ConnectionStatus::Connected;
        info!(
            "✅ WireGuard server up: iface={}, listen={}, peers={} ({} in UAPI), handle={}",
            iface,
            wg.interface.listen_port,
            wg.peers.len(),
            wg_uapi.peers.len(),
            handle
        );
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    {
        if let Err(e) = tun.configure_address(&tunnel_addrs) {
            let _ = tun.close();
            return Err(format!(
                "Configure {} on {} failed: {}",
                tunnel_addrs, iface, e
            ));
        }
        if let Err(e) = ensure_server_routes(&iface, &tunnel_addrs) {
            warn!("Failed to ensure server routes on {}: {}", iface, e);
        }

        let session_fd = tun.fd;
        let cleanup_fd = dup_tun_for_teardown(session_fd)?;
        state.tun_fd = Some(cleanup_fd);

        log::debug!("uapi: {}", uapi);
        // 2. Hand to libwg-go.
        let handle = match WireGuardApi::turn_on(&uapi, session_fd) {
            Ok(h) => h,
            Err(e) => {
                close_fd_best_effort("unix wgTurnOn err (session)", session_fd);
                if let Some(c) = state.tun_fd.take() {
                    close_fd_best_effort("unix wgTurnOn err (dup)", c);
                }
                state.status = ConnectionStatus::Error;
                return Err(format!("wgTurnOn failed: {}", e));
            }
        };

        state.handle = Some(handle);
        state.status = ConnectionStatus::Connected;
        info!(
            "✅ WireGuard server up: iface={}, listen={}, peers={} ({} in UAPI), handle={}",
            iface,
            wg.interface.listen_port,
            wg.peers.len(),
            wg_uapi.peers.len(),
            handle
        );
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        // Same ordering as Linux: wgTurnOn first, then OS IPv4 / MTU (Wintun + netsh).
        let session_fd = tun.fd;
        let cleanup_fd = dup_tun_for_teardown(session_fd)?;
        state.tun_fd = Some(cleanup_fd);
        log::debug!("uapi: {}", uapi);
        let handle = match WireGuardApi::turn_on(&uapi, session_fd) {
            Ok(h) => h,
            Err(e) => {
                close_fd_best_effort("windows wgTurnOn err (session)", session_fd);
                if let Some(c) = state.tun_fd.take() {
                    close_fd_best_effort("windows wgTurnOn err (dup)", c);
                }
                state.status = ConnectionStatus::Error;
                return Err(format!("wgTurnOn failed: {}", e));
            }
        };
        if let Err(e) = tun.configure_address(&tunnel_addrs) {
            WireGuardApi::turn_off(handle);
            if let Some(c) = state.tun_fd.take() {
                close_fd_best_effort("windows configure_addr (dup)", c);
            }
            state.status = ConnectionStatus::Error;
            return Err(format!(
                "Configure {} on {} failed: {}",
                tunnel_addrs, iface, e
            ));
        }
        if let Err(e) = tun.set_mtu(mtu) {
            warn!("Windows MTU on {}: {}", iface, e);
        }
        state.handle = Some(handle);
        state.status = ConnectionStatus::Connected;
        info!(
            "✅ WireGuard server up: iface={}, listen={}, peers={} ({} in UAPI), handle={}",
            iface,
            wg.interface.listen_port,
            wg.peers.len(),
            wg_uapi.peers.len(),
            handle
        );
        Ok(())
    }

    #[cfg(all(not(unix), not(target_os = "windows")))]
    {
        drop(tun);
        Err("WireGuard server TUN is not implemented on this platform.".to_string())
    }
}

/// Duplicate the TUN fd immediately after creation; pass the original to [`WireGuardApi::turn_on`]
/// and store the copy in [`WireGuardState::tun_fd`]. `stop` closes only the duplicate so we never
/// `close(2)` the same integer the library may have already closed (which can hit a recycled fd).
#[cfg(any(unix, target_os = "windows"))]
fn dup_tun_for_teardown(session_fd: i32) -> Result<i32, String> {
    if session_fd < 0 {
        return Err("invalid TUN file descriptor".to_string());
    }
    let cleanup_fd = unsafe { libc::dup(session_fd) };
    if cleanup_fd < 0 {
        return Err(format!(
            "dup(TUN fd): {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(cleanup_fd)
}

/// Close `fd` if it is still open (skip if EBADF — e.g. already closed by libwg-go).
#[cfg(unix)]
fn close_fd_best_effort(label: &str, fd: i32) {
    if fd < 0 {
        return;
    }
    unsafe {
        if libc::fcntl(fd, libc::F_GETFD) < 0 {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::EBADF) {
                warn!("{}: fcntl(F_GETFD) on fd {}: {}", label, fd, e);
            }
            return;
        }
        if libc::close(fd) < 0 {
            warn!(
                "{}: close({}): {}",
                label,
                fd,
                std::io::Error::last_os_error()
            );
        }
    }
}

#[cfg(target_os = "windows")]
fn close_fd_best_effort(label: &str, fd: i32) {
    if fd < 0 {
        return;
    }
    unsafe {
        if libc::close(fd) < 0 {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() != Some(libc::EBADF) {
                warn!("{}: close({}): {}", label, fd, e);
            }
        }
    }
}

/// Stop the WireGuard server: tear down userspace, release kernel TUN, scrub routes.
///
/// After [`WireGuardApi::turn_off`], close the **dup** held in [`WireGuardState::tun_fd`] so the
/// kernel drops the last reference if libwg-go already closed the session fd. Then best-effort
/// `ip link del` / `ifconfig down` so `InterfaceName` can be recreated.
pub fn stop(state: &mut WireGuardState) -> Result<(), String> {
    if state.app_mode != AppMode::Server {
        return Err("server::stop called outside server mode".to_string());
    }

    let iface_opt = read_env_identity()
        .ok()
        .and_then(|(_, _, _, n)| n.filter(|s| !s.is_empty()));

    // 1. Userspace WireGuard (Amnezia/libwg-go).
    if let Some(handle) = state.handle.take() {
        WireGuardApi::turn_off(handle);
        info!("WireGuard tunnel turned off (handle={})", handle);
    }

    // 2. Close the teardown dup (not the session fd handed to wgTurnOn).
    #[cfg(any(unix, target_os = "windows"))]
    if let Some(fd) = state.tun_fd.take() {
        close_fd_best_effort("server stop (tun dup)", fd);
    }
    #[cfg(all(not(unix), not(target_os = "windows")))]
    {
        state.tun_fd = None;
    }

    // 3. Routes (macOS) and link teardown so the next start gets a clean slate.
    if let Some(ref iface_name) = iface_opt {
        let addr = state
            .wg_config
            .as_ref()
            .map(|w| w.interface.address.clone())
            .unwrap_or_else(|| "10.88.0.1/24".to_string());
        if let Err(e) = cleanup_server_routes(iface_name, &addr) {
            warn!("Failed to cleanup server routes on {}: {}", iface_name, e);
        }
        best_effort_bring_tun_down(iface_name);
    }

    state.status = ConnectionStatus::Disconnected;
    Ok(())
}

/// Best-effort `ip link down` / `delete` (Linux) or `ifconfig down` (macOS).
fn best_effort_bring_tun_down(iface: &str) {
    if iface.is_empty() {
        return;
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("ip")
            .args(["link", "set", "dev", iface, "down"])
            .output();
        let _ = std::process::Command::new("ip")
            .args(["link", "delete", "dev", iface])
            .output();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("ifconfig")
            .args([iface, "down"])
            .output();
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = iface;
    }
}

/// Best-effort throughput counters for the running server.
pub fn stats(state: &WireGuardState) -> (u64, u64) {
    if let Some(handle) = state.handle {
        match WireGuardApi::get_stats(handle) {
            Ok((rx, tx)) => return (rx, tx),
            Err(e) => warn!("wgGetStats failed: {}", e),
        }
    }
    (0, 0)
}

/// Full in-memory peer list with interface settings, minus peers soft-disabled for the device.
pub fn wg_config_for_device_uapi(state: &WireGuardState, wg: &Wg) -> Wg {
    let mut w = wg.clone();
    w.peers = w
        .peers
        .iter()
        .filter(|p| !state.server_runtime_excluded_pubkeys.contains(&p.public_key))
        .cloned()
        .collect();
    w
}

/// Apply a hot-reload of the peer list to a running server. Caller is
/// responsible for persisting via `wg::store::save_peers` first.
///
/// Always pushes a **full** UAPI from [`build_uapi`] (including `replace_peers=true`); peers in
/// [`WireGuardState::server_runtime_excluded_pubkeys`] are omitted from the device config only.
pub fn apply_peers(state: &mut WireGuardState, new_peers: &[Peer]) -> Result<(), String> {
    let mut wg = state.wg_config.clone().unwrap_or_default();
    wg.peers = new_peers.to_vec();
    state.wg_config = Some(wg.clone());

    if let Some(handle) = state.handle {
        let wg_uapi = wg_config_for_device_uapi(state, &wg);
        let uapi = build_uapi(&wg_uapi)?;
        if let Err(rc) = WireGuardApi::set_config(handle, &uapi) {
            return Err(format!("wgSetConfig returned {}", rc));
        }
        info!(
            "Hot-reloaded {} configured peers ({} in UAPI) into running server (handle={})",
            new_peers.len(),
            wg_uapi.peers.len(),
            handle
        );
    }

    Ok(())
}

/// UI hint: whether each configured peer is included in the live tunnel (not soft-disabled).
/// When the server is not running, all entries are `true` (toggles are inert until start).
pub fn peers_wg_runtime_active(state: &WireGuardState, peers: &[Peer]) -> Vec<bool> {
    if state.handle.is_none() {
        return vec![true; peers.len()];
    }
    peers
        .iter()
        .map(|p| !state.server_runtime_excluded_pubkeys.contains(&p.public_key))
        .collect()
}

// ---------------------------------------------------------------------------
// .env loader (kept here so all server-side knowledge of the file is in one
// place). Backward compatible with the existing layout in main.rs.
// ---------------------------------------------------------------------------

/// UDP port suffix from `.env` `Endpoint` (`1.2.3.4:51820`, `host:51820`, `[::1]:51820`).
fn port_from_endpoint(endpoint: &str) -> Option<u16> {
    let e = endpoint.trim();
    if e.is_empty() {
        return None;
    }
    let port_str = if let Some(rest) = e.strip_prefix('[') {
        rest.split_once("]:")?.1
    } else {
        e.rsplit_once(':')?.1
    };
    port_str.parse().ok()
}

fn read_env_identity() -> Result<(String, Option<u16>, Option<String>, Option<String>), String> {
    let content = match std::fs::read_to_string(".env") {
        Ok(c) => c,
        Err(e) => return Err(format!("read .env: {}", e)),
    };

    let mut private_key = String::new();
    let mut listen_port: Option<u16> = None;
    let mut endpoint: Option<String> = None;
    let mut interface_name: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (k, v) = match trimmed.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"').trim_matches('\'');

        match k {
            "PrivateKey" => private_key = v.to_string(),
            "InterfaceName" => {
                if !v.is_empty() {
                    interface_name = Some(v.to_string());
                }
            }
            "ListenPort" => {
                if let Ok(p) = v.parse::<u16>() {
                    listen_port = Some(p);
                }
            }
            "Endpoint" => {
                if !v.is_empty() {
                    endpoint = Some(v.to_string());
                }
            }
            _ => {}
        }
    }

    if private_key.is_empty() {
        warn!(".env present but missing PrivateKey=... line");
    }
    Ok((private_key, listen_port, endpoint, interface_name))
}

fn read_env_amnezia_params() -> Result<Option<AmneziaParams>, String> {
    let content = std::fs::read_to_string(".env").map_err(|e| format!("read .env: {}", e))?;

    let mut jc: Option<u16> = None;
    let mut jmin: Option<u16> = None;
    let mut jmax: Option<u16> = None;
    let mut s1: Option<u16> = None;
    let mut s2: Option<u16> = None;
    let mut h1: Option<u16> = None;
    let mut h2: Option<u16> = None;
    let mut h3: Option<u16> = None;
    let mut h4: Option<u16> = None;
    let mut i1: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let (k, v) = match trimmed.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        let key = k.trim().to_ascii_lowercase();
        let value = v.trim().trim_matches('"').trim_matches('\'');
        match key.as_str() {
            "jc" => jc = value.parse::<u16>().ok(),
            "jmin" => jmin = value.parse::<u16>().ok(),
            "jmax" => jmax = value.parse::<u16>().ok(),
            "s1" => s1 = value.parse::<u16>().ok(),
            "s2" => s2 = value.parse::<u16>().ok(),
            "h1" => h1 = value.parse::<u16>().ok(),
            "h2" => h2 = value.parse::<u16>().ok(),
            "h3" => h3 = value.parse::<u16>().ok(),
            "h4" => h4 = value.parse::<u16>().ok(),
            "i1" => i1 = Some(value.to_string()),
            _ => {}
        }
    }

    if jc.is_none()
        && jmin.is_none()
        && jmax.is_none()
        && s1.is_none()
        && s2.is_none()
        && h1.is_none()
        && h2.is_none()
        && h3.is_none()
        && h4.is_none()
        && i1.is_none()
    {
        return Ok(None);
    }

    Ok(Some(AmneziaParams {
        jc: jc.unwrap_or(0),
        jmin: jmin.unwrap_or(0),
        jmax: jmax.unwrap_or(0),
        s1: s1.unwrap_or(0),
        s2: s2.unwrap_or(0),
        h1: h1.unwrap_or(0),
        h2: h2.unwrap_or(0),
        h3: h3.unwrap_or(0),
        h4: h4.unwrap_or(0),
        i1: i1.unwrap_or_default(),
    }))
}

fn base64_to_hex(input: &str) -> Result<String, String> {
    use base64::{engine::general_purpose, Engine as _};
    let bytes = general_purpose::STANDARD
        .decode(input.trim())
        .map_err(|e| format!("base64 decode '{}': {}", input, e))?;
    Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
}

#[cfg(target_os = "macos")]
fn ensure_server_routes(iface: &str, addr_list: &str) -> Result<(), String> {
    for cidr in addr_list.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Some(net_cidr) = ipv4_network_cidr(cidr) {
            // Keep VPN subnet traffic on utun (e.g. 10.88.0.0/16 -> utun9981).
            let _ = Command::new("route")
                .args(["-n", "delete", "-net", &net_cidr])
                .output();
            let out = Command::new("route")
                .args(["-n", "add", "-net", &net_cidr, "-interface", iface])
                .output()
                .map_err(|e| format!("route add {} via {} failed: {}", net_cidr, iface, e))?;
            if !out.status.success() {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                return Err(format!("route add {} via {} failed: {}", net_cidr, iface, err));
            }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn ensure_server_routes(_iface: &str, _addr_list: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn cleanup_server_routes(iface: &str, addr_list: &str) -> Result<(), String> {
    for cidr in addr_list.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Some(net_cidr) = ipv4_network_cidr(cidr) {
            let _ = Command::new("route")
                .args(["-n", "delete", "-net", &net_cidr, "-interface", iface])
                .output()
                .map_err(|e| format!("route delete {} via {} failed: {}", net_cidr, iface, e))?;
        }
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn cleanup_server_routes(_iface: &str, _addr_list: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn ipv4_network_cidr(cidr: &str) -> Option<String> {
    let (ip_str, prefix_str) = cidr.split_once('/')?;
    let ip: Ipv4Addr = ip_str.trim().parse().ok()?;
    let prefix: u32 = prefix_str.trim().parse().ok()?;
    if prefix > 32 {
        return None;
    }
    let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
    let network = u32::from(ip) & mask;
    Some(format!("{}/{}", Ipv4Addr::from(network), prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn priv_b64() -> &'static str {
        "yMlj3LbVKMW69kXXh0OpbfZUlEVmkYDao3bk6jTl/EQ="
    }
    fn pub_b64() -> &'static str {
        "qHaIfS7u47/U1AuigBDhOv/p/t6Gy+XKSUdYnPIEKDA="
    }

    #[test]
    fn port_from_endpoint_parses_host_and_ipv6() {
        assert_eq!(port_from_endpoint("203.0.113.7:51820"), Some(51820));
        assert_eq!(port_from_endpoint("vpn.example.com:41194"), Some(41194));
        assert_eq!(port_from_endpoint("[2001:db8::1]:51820"), Some(51820));
        assert_eq!(port_from_endpoint("no-port"), None);
        assert_eq!(port_from_endpoint(""), None);
    }

    #[test]
    fn build_uapi_clears_h_params() {
        let wg = Wg {
            interface: crate::wg::config::Interface {
                private_key: priv_b64().into(),
                listen_port: 51820,
                address: "10.88.0.1/24".into(),
                dns: vec![],
                mtu: 1280,
                amnezia_params: None,
            },
            peers: vec![Peer {
                name: Some("client1".into()),
                private_key: None,
                public_key: pub_b64().into(),
                preshared_key: String::new(),
                allowed_ips: vec!["10.88.0.2/32".into()],
                endpoint: String::new(),
                persistent_keepalive: None,
            }],
        };
        let uapi = build_uapi(&wg).expect("uapi");

        // Must contain core lines.
        assert!(uapi.contains("listen_port=61820"));
        assert!(uapi.contains("replace_peers=true"));
        assert!(uapi.contains("allowed_ip=10.88.0.2/32"));
        assert!(uapi.contains("private_key="));
        assert!(uapi.contains("public_key="));
        println!("uapi: {}", uapi);
        for forbidden in ["jc=", "jmin=", "jmax=", "s1=", "s2=", "h1=", "h2=", "h3=", "h4=", "i1="] {
            assert!(
                !uapi.contains(forbidden),
                "server uapi must not contain '{}', got:\n{}",
                forbidden,
                uapi
            );
        }
    }

    #[test]
    fn build_uapi_omits_peer_endpoint() {
        let wg = Wg {
            interface: crate::wg::config::Interface {
                private_key: priv_b64().into(),
                listen_port: 51820,
                address: "10.88.0.1/24".into(),
                dns: vec![],
                mtu: 1280,
                amnezia_params: None,
            },
            peers: vec![Peer {
                name: Some("c".into()),
                private_key: None,
                public_key: pub_b64().into(),
                preshared_key: String::new(),
                allowed_ips: vec!["10.88.0.2/32".into()],
                endpoint: "192.168.6.103:61820".into(),
                persistent_keepalive: Some(25),
            }],
        };
        let uapi = build_uapi(&wg).expect("uapi");
        assert!(
            !uapi.contains("endpoint="),
            "server uapi must not contain peer endpoint=, got:\n{}",
            uapi
        );
        // assert!(
        //     uapi.contains("persistent_keepalive_interval="),
        //     "expected keepalive in uapi"
        // );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn normalize_tunnel_addr_bare_ipv4_appends_slash24() {
        assert_eq!(
            super::normalize_linux_server_tunnel_address_list("10.88.0.1"),
            "10.88.0.1/24"
        );
        assert_eq!(
            super::normalize_linux_server_tunnel_address_list("10.88.0.1/24"),
            "10.88.0.1/24"
        );
        assert_eq!(
            super::normalize_linux_server_tunnel_address_list("10.88.0.1/32"),
            "10.88.0.1/32"
        );
        assert_eq!(
            super::normalize_linux_server_tunnel_address_list("fd00::1"),
            "fd00::1"
        );
    }

    #[test]
    fn build_uapi_rejects_empty_private_key() {
        let wg = Wg::default();
        let err = build_uapi(&wg).unwrap_err();
        assert!(err.contains("PrivateKey"), "unexpected error: {}", err);
    }
}
