//! Cross-platform TUN device abstraction used by WireGuard server (and, in
//! the future, client) mode.
//!
//! - **macOS**: delegates creation to libwg-go's `createTun` (utun via
//!   PF_SYSTEM, already linked into our binary). Address/MTU configuration
//!   shells out to `ifconfig`.
//! - **Linux (server)**: use **libwg-go `createTun`** — same as macOS. Opening
//!   `/dev/net/tun` via Rust `TUNSETIFF` produced fds that still triggered
//!   **VirtualTun** fallback (`setMTU` EINVAL / `write virtual-tun: destination address required`).
//!   IP configuration still uses the `ip` tool in [`configure_address`].
//! - **Windows (server)**: **libwg-go `createTun`** (Wintun). Place **wintun.dll**
//!   next to the executable. IPv4 is configured with **netsh** after `wgTurnOn`.

use log::{debug, info, warn};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use log::error;
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;
#[cfg(target_os = "windows")]
use crate::command_ext::command_new;

/// Owns a TUN device file descriptor (or, on Windows, would own a wintun
/// session handle). Closing is explicit via [`TunDevice::close`]; if the
/// caller drops without closing the fd is leaked deliberately so callers
/// can hand the fd to libwg-go which manages its lifetime afterwards.
#[derive(Debug)]
pub struct TunDevice {
    pub name: String,
    /// On macOS/Linux this is the kernel file descriptor for the tun socket.
    /// libwg-go takes ownership of the fd once `wgTurnOn` is called.
    pub fd: i32,
    pub mtu: u16,
}

impl TunDevice {
    /// Allocate a new TUN device with the given name and MTU.
    ///
    /// On macOS the name MUST start with `utun` and end with a non-negative
    /// integer (libwg-go enforces this).
    ///
    /// On Linux the device is created in non-persistent mode; once the fd is
    /// closed the kernel removes the interface.
    pub fn create(name: &str, mtu: u16) -> Result<Self, String> {
        info!("Creating TUN device '{}' (mtu={})", name, mtu);

        #[cfg(target_os = "macos")]
        {
            // Reuse libwg-go's utun creator (already statically linked).
            let fd = crate::wg::WireGuardApi::create_tun(name, mtu as i32)?;
            // Set MTU via ifconfig in case the default differs.
            let _ = run_ok(
                "ifconfig",
                &[name, "mtu", &mtu.to_string()],
                "ifconfig mtu",
            );
            Ok(TunDevice {
                name: name.to_string(),
                fd,
                mtu,
            })
        }

        #[cfg(target_os = "linux")]
        {
            // Must match how the linked **libwg-go** opens TUN (`CreateTUN`). A Rust-only
            // `TUNSETIFF` fd still led to VirtualTun + broken `write(2)` on some kernels.
            let fd = crate::wg::WireGuardApi::create_tun(name, mtu as i32)?;
            Ok(TunDevice {
                name: name.to_string(),
                fd,
                mtu,
            })
        }

        #[cfg(target_os = "windows")]
        {
            let fd = crate::wg::WireGuardApi::create_tun(name, mtu as i32)?;
            Ok(TunDevice {
                name: name.to_string(),
                fd,
                mtu,
            })
        }
    }

    /// Configure an IPv4 (and optionally IPv6) address on the TUN device.
    /// Accepts CIDR notation, e.g. `10.88.0.1/24` or `fd00::1/64`. Multiple
    /// addresses can be passed comma-separated.
    pub fn configure_address(&self, addr_with_cidr: &str) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            // One flush before all adds: avoids per-address flush wiping prior `ip addr add`s,
            // and clears stale `/32`+`/24` duplicates from earlier runs/tools.
            let _ = run_ok(
                "ip",
                &["-4", "addr", "flush", "dev", &self.name],
                "ip -4 addr flush",
            );
            let _ = run_ok(
                "ip",
                &["-6", "addr", "flush", "dev", &self.name],
                "ip -6 addr flush",
            );
        }
        #[cfg(target_os = "windows")]
        {
            let parts: Vec<&str> = addr_with_cidr
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect();
            for (i, addr) in parts.iter().enumerate() {
                windows_apply_address_on_iface(&self.name, addr, i == 0)?;
            }
            return Ok(());
        }

        #[cfg(not(target_os = "windows"))]
        {
            for raw in addr_with_cidr.split(',') {
                let addr = raw.trim();
                if addr.is_empty() {
                    continue;
                }
                self.configure_one_address(addr)?;
            }
            Ok(())
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn configure_one_address(&self, addr: &str) -> Result<(), String> {
        let is_ipv6 = addr.contains(':');
        let (ip_only, prefix) = match addr.split_once('/') {
            Some((ip, p)) => (ip, p.to_string()),
            None => (
                addr,
                if is_ipv6 { "128".to_string() } else { "32".to_string() },
            ),
        };

        debug!(
            "Configuring address {}/{} on {} (ipv6={})",
            ip_only, prefix, self.name, is_ipv6
        );

        #[cfg(target_os = "macos")]
        {
            // macOS utun uses point-to-point form for IPv4. Keep local/peer the
            // same, and rely on explicit subnet routes managed by server logic.
            if is_ipv6 {
                run_required(
                    "ifconfig",
                    &[
                        &self.name,
                        "inet6",
                        ip_only,
                        ip_only,
                        "prefixlen",
                        &prefix,
                    ],
                    "ifconfig inet6",
                )?;
            } else {
                let netmask = prefix_to_netmask_v4(&prefix)?;
                run_required(
                    "ifconfig",
                    &[
                        &self.name,
                        "inet",
                        ip_only,
                        ip_only,
                        "netmask",
                        &netmask,
                    ],
                    "ifconfig inet",
                )?;
            }
            Ok(())
        }

        #[cfg(target_os = "linux")]
        {
            let cidr = format!("{}/{}", ip_only, prefix);
            run_required(
                "ip",
                &["addr", "add", &cidr, "dev", &self.name],
                "ip addr add",
            )
        }
    }

    /// Adjust MTU after creation if needed.
    pub fn set_mtu(&self, mtu: u16) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        {
            run_required(
                "ifconfig",
                &[&self.name, "mtu", &mtu.to_string()],
                "ifconfig mtu",
            )
        }
        #[cfg(target_os = "linux")]
        {
            run_required(
                "ip",
                &["link", "set", "dev", &self.name, "mtu", &mtu.to_string()],
                "ip link set mtu",
            )
        }
        #[cfg(target_os = "windows")]
        {
            match windows_netsh_mtu_ok(&self.name, mtu) {
                Ok(()) => Ok(()),
                Err(e) => {
                    warn!(
                        "[tun] netsh mtu on {} (tunnel may still use default MTU): {}",
                        self.name, e
                    );
                    Ok(())
                }
            }
        }
    }

    /// Linux only: apply `mtu` and `UP` via `ip` **after** `wgTurnOn` has attached to the tun fd.
    #[cfg(target_os = "linux")]
    pub(crate) fn linux_apply_mtu_up(&self, mtu: u16) -> Result<(), String> {
        run_required(
            "ip",
            &["link", "set", "dev", &self.name, "mtu", &mtu.to_string()],
            "ip link set mtu (post-wgTurnOn)",
        )?;
        run_required(
            "ip",
            &["link", "set", "dev", &self.name, "up"],
            "ip link set up (post-wgTurnOn)",
        )?;
        Ok(())
    }

    /// Close the underlying file descriptor and (on Linux) tear the
    /// interface down. After this the [`TunDevice`] should not be used.
    pub fn close(self) -> Result<(), String> {
        info!("Closing TUN device '{}' (fd={})", self.name, self.fd);

        #[cfg(target_os = "linux")]
        {
            // Take the interface down before closing the fd; ignore failures.
            let _ = run_ok(
                "ip",
                &["link", "set", "dev", &self.name, "down"],
                "ip link set down",
            );
        }

        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            if self.fd > 0 {
                let rc = unsafe { libc::close(self.fd) };
                if rc != 0 {
                    let err = std::io::Error::last_os_error();
                    warn!(
                        "close({}) on TUN '{}' returned errno {}: {}",
                        self.fd, self.name, err.raw_os_error().unwrap_or(-1), err
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn prefix_to_netmask_v4_windows(prefix: &str) -> Result<String, String> {
    let p: u32 = prefix
        .parse()
        .map_err(|e| format!("Invalid IPv4 prefix '{}': {}", prefix, e))?;
    if p > 32 {
        return Err(format!("IPv4 prefix '{}' out of range", prefix));
    }
    let mask: u32 = if p == 0 { 0 } else { 0xFFFF_FFFFu32 << (32 - p) };
    Ok(format!(
        "{}.{}.{}.{}",
        (mask >> 24) & 0xff,
        (mask >> 16) & 0xff,
        (mask >> 8) & 0xff,
        mask & 0xff
    ))
}

#[cfg(target_os = "windows")]
fn windows_apply_address_on_iface(iface: &str, addr: &str, is_first: bool) -> Result<(), String> {
    let is_ipv6 = addr.contains(':');
    let (ip_only, prefix) = match addr.split_once('/') {
        Some((ip, p)) => (ip.trim(), p.to_string()),
        None => (
            addr.trim(),
            if is_ipv6 {
                "128".to_string()
            } else {
                "32".to_string()
            },
        ),
    };
    if is_ipv6 {
        warn!(
            "[tun] Skipping IPv6 address {} on {} (server Windows path is IPv4-only)",
            addr, iface
        );
        return Ok(());
    }
    let mask = prefix_to_netmask_v4_windows(&prefix)?;
    let name_arg = format!("name=\"{}\"", iface);
    let mut cmd = command_new("netsh");
    cmd.arg("interface").arg("ip");
    if is_first {
        cmd.arg("set")
            .arg("address")
            .arg(&name_arg)
            .arg("source=static")
            .arg(format!("addr={}", ip_only))
            .arg(format!("mask={}", mask));
    } else {
        cmd.arg("add")
            .arg("address")
            .arg(&name_arg)
            .arg(format!("addr={}", ip_only))
            .arg(format!("mask={}", mask));
    }
    debug!("[tun] netsh {:?}", cmd);
    let out = cmd
        .output()
        .map_err(|e| format!("netsh address spawn: {}", e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let stdout = String::from_utf8_lossy(&out.stdout);
        return Err(format!(
            "netsh address on {}: status={} stderr={} stdout={}",
            iface,
            out.status,
            stderr.trim(),
            stdout.trim()
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_netsh_mtu_ok(iface: &str, mtu: u16) -> Result<(), String> {
    let name_arg = format!("interface=\"{}\"", iface);
    let mtu_arg = format!("mtu={}", mtu);
    let out = command_new("netsh")
        .arg("interface")
        .arg("ipv4")
        .arg("set")
        .arg("subinterface")
        .arg(&name_arg)
        .arg(&mtu_arg)
        .arg("store=active")
        .output()
        .map_err(|e| format!("netsh mtu spawn: {}", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn prefix_to_netmask_v4(prefix: &str) -> Result<String, String> {
    let p: u32 = prefix
        .parse()
        .map_err(|e| format!("Invalid IPv4 prefix '{}': {}", prefix, e))?;
    if p > 32 {
        return Err(format!("IPv4 prefix '{}' out of range", prefix));
    }
    let mask: u32 = if p == 0 { 0 } else { 0xFFFF_FFFFu32 << (32 - p) };
    Ok(format!(
        "{}.{}.{}.{}",
        (mask >> 24) & 0xff,
        (mask >> 16) & 0xff,
        (mask >> 8) & 0xff,
        mask & 0xff
    ))
}

/// Run a command, returning Err on non-zero exit. Used for steps that must
/// succeed for the device to be usable.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run_required(cmd: &str, args: &[&str], label: &str) -> Result<(), String> {
    debug!("[tun] {} -> {} {:?}", label, cmd, args);
    let out = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("{} ({}) failed to spawn: {}", label, cmd, e))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "{} ({} {:?}) exited with status {}: {}",
            label,
            cmd,
            args,
            out.status,
            stderr.trim()
        ));
    }
    Ok(())
}

/// Best-effort run; logs but never fails.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run_ok(cmd: &str, args: &[&str], label: &str) -> bool {
    debug!("[tun] {} -> {} {:?}", label, cmd, args);
    match Command::new(cmd).args(args).output() {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                warn!("{} ({:?}) failed: {}", label, args, stderr.trim());
                return false;
            }
            true
        }
        Err(e) => {
            error!("{} ({}) spawn error: {}", label, cmd, e);
            false
        }
    }
}
