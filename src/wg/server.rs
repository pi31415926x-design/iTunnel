//! WireGuard server-mode implementation.
//!
//! Responsibilities:
//!
//! 1. Build a server [`Wg`] by combining the on-disk peer list
//!    (`~/.itunnel/itunnel_peers.json`) with the operator-supplied identity
//!    in `.env` (`PrivateKey=...`, `InterfaceName=...` for the TUN device name,
//!    optional `ListenPort=...`, optional `Endpoint=...`).
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
//! - Windows server mode is not yet supported (TUN layer is stubbed).

use crate::wg::config::{AmneziaParams, AppMode, ConnectionStatus, Peer, Wg, WireGuardState};
use crate::wg::tun::TunDevice;
use crate::wg::WireGuardApi;
use log::{info, debug, warn};
use std::net::Ipv4Addr;
use std::process::Command;

/// Default MTU for the server TUN. 1280 is the typical conservative WG MTU
/// (matches what client mode uses). Operators can override via the INI
/// `MTU =` field once we add INI loading.
const DEFAULT_SERVER_MTU: u16 = 1280;

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
    match read_env_identity() {
        Ok((pk, port_opt, endpoint_opt, ifn)) => {
            interface_name = ifn.filter(|s| !s.is_empty());
            if wg.interface.private_key.is_empty() || wg.interface.listen_port == 0 {
                if !pk.is_empty() && wg.interface.private_key.is_empty() {
                    wg.interface.private_key = pk;
                }
                if let Some(port) = port_opt {
                    if wg.interface.listen_port == 0 {
                        wg.interface.listen_port = port;
                    }
                }
                if let Some(ep) = endpoint_opt {
                    state.active_endpoint = Some(ep);
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
        // Server peers are typically roaming clients; an Endpoint is optional
        // and only meaningful as a persistent override.
        if !p.endpoint.is_empty() {
            s.push_str(&format!("endpoint={}\n", p.endpoint));
        }
        if let Some(ka) = p.persistent_keepalive {
            if ka > 0 {
                s.push_str(&format!("persistent_keepalive_interval={}\n", ka));
            }
        }
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
    let uapi = build_uapi(&wg)?;

    // 1. TUN (name from .env: InterfaceName=...)
    let iface = tun_name_opt
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            "Server TUN name is not set. Add InterfaceName=... to the project root .env (e.g. \
             utun88 on macOS, wg0 on Linux)."
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

    if let Err(e) = tun.configure_address(&wg.interface.address) {
        let _ = tun.close();
        return Err(format!(
            "Configure {} on {} failed: {}",
            wg.interface.address, iface, e
        ));
    }
    if let Err(e) = ensure_server_routes(&iface, &wg.interface.address) {
        warn!("Failed to ensure server routes on {}: {}", iface, e);
    }

    let fd = tun.fd;
    state.tun_fd = Some(fd);

    debug!("uapi: {}", uapi);
    // 2. Hand to libwg-go.
    let handle = match WireGuardApi::turn_on(&uapi, fd) {
        Ok(h) => h,
        Err(e) => {
            // libwg-go assumes ownership of the fd ONLY on success; close it
            // here. NB: do not call tun.close() because that would also try
            // to close it again. Fall back to the raw libc close.
            unsafe { libc::close(fd) };
            state.tun_fd = None;
            state.status = ConnectionStatus::Error;
            return Err(format!("wgTurnOn failed: {}", e));
        }
    };

    state.handle = Some(handle);
    state.status = ConnectionStatus::Connected;
    info!(
        "✅ WireGuard server up: iface={}, listen={}, peers={}, handle={}",
        iface,
        wg.interface.listen_port,
        wg.peers.len(),
        handle
    );

    Ok(())
}

/// Stop the WireGuard server: turn off libwg-go, close TUN.
pub fn stop(state: &mut WireGuardState) -> Result<(), String> {
    if state.app_mode != AppMode::Server {
        return Err("server::stop called outside server mode".to_string());
    }

    // 1. WireGuard tunnel.
    if let Some(handle) = state.handle.take() {
        WireGuardApi::turn_off(handle);
        info!("WireGuard tunnel turned off (handle={})", handle);
    }

    // 2. TUN. libwg-go closes the fd it owns on `wgTurnOff`, so we don't
    //    re-close here unless it was never handed off.
    state.tun_fd = None;
    if let Ok((_, _, _, Some(iface_name))) = read_env_identity() {
        let addr = state
            .wg_config
            .as_ref()
            .map(|w| w.interface.address.clone())
            .unwrap_or_else(|| "10.88.0.1/24".to_string());
        if let Err(e) = cleanup_server_routes(&iface_name, &addr) {
            warn!("Failed to cleanup server routes on {}: {}", iface_name, e);
        }
    }

    state.status = ConnectionStatus::Disconnected;
    Ok(())
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

/// Apply a hot-reload of the peer list to a running server. Caller is
/// responsible for persisting via `wg::store::save_peers` first.
pub fn apply_peers(state: &mut WireGuardState, new_peers: &[Peer]) -> Result<(), String> {
    let mut wg = state.wg_config.clone().unwrap_or_default();
    wg.peers = new_peers.to_vec();
    state.wg_config = Some(wg.clone());

    if let Some(handle) = state.handle {
        let uapi = build_uapi(&wg)?;
        if let Err(rc) = WireGuardApi::set_config(handle, &uapi) {
            return Err(format!("wgSetConfig returned {}", rc));
        }
        info!(
            "Hot-reloaded {} peers into running server (handle={})",
            new_peers.len(),
            handle
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// .env loader (kept here so all server-side knowledge of the file is in one
// place). Backward compatible with the existing layout in main.rs.
// ---------------------------------------------------------------------------

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
                    // Endpoint of the form host:port also gives us the listen port.
                    if listen_port.is_none() {
                        if let Some(port_str) = v.rsplit(':').next() {
                            if let Ok(p) = port_str.parse::<u16>() {
                                listen_port = Some(p);
                            }
                        }
                    }
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
    fn build_uapi_clears_h_params() {
        let wg = Wg {
            interface: crate::wg::config::Interface {
                private_key: priv_b64().into(),
                listen_port: 51820,
                address: "10.88.0.1/24".into(),
                dns: vec![],
                mtu: 1280,
                amnezia_params: todo!(),
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
        assert!(uapi.contains("listen_port=51820"));
        assert!(uapi.contains("replace_peers=true"));
        assert!(uapi.contains("allowed_ip=10.88.0.2/32"));
        assert!(uapi.contains("private_key="));
        assert!(uapi.contains("public_key="));
        println!("uapi: {}", uapi);
        // Server explicitly clears h1..h4 while leaving other knobs absent.
        assert!(uapi.contains("h1=0"));
        assert!(uapi.contains("h2=0"));
        assert!(uapi.contains("h3=0"));
        assert!(uapi.contains("h4=0"));
        for forbidden in ["jc=", "jmin=", "jmax=", "s1=", "s2=", "i1="] {
            assert!(
                !uapi.contains(forbidden),
                "server uapi must not contain '{}', got:\n{}",
                forbidden,
                uapi
            );
        }
    }

    #[test]
    fn build_uapi_rejects_empty_private_key() {
        let wg = Wg::default();
        let err = build_uapi(&wg).unwrap_err();
        assert!(err.contains("PrivateKey"), "unexpected error: {}", err);
    }
}
