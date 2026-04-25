//! Cross-platform TUN device abstraction used by WireGuard server (and, in
//! the future, client) mode.
//!
//! - **macOS**: delegates creation to libwg-go's `createTun` (utun via
//!   PF_SYSTEM, already linked into our binary). Address/MTU configuration
//!   shells out to `ifconfig`.
//! - **Linux**: native Rust using `/dev/net/tun` + `TUNSETIFF`. Address/MTU
//!   configuration shells out to the modern `ip` tool.
//! - **Windows**: not yet implemented (wintun.dll not bundled). Returns an
//!   error so the rest of the system degrades gracefully.

use log::{debug, error, info, warn};
use std::process::Command;

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
            let fd = linux::open_tun(name)?;
            // Bring the link up and set MTU. Address is configured separately.
            run_required(
                "ip",
                &["link", "set", "dev", name, "mtu", &mtu.to_string()],
                "ip link set mtu",
            )?;
            run_required(
                "ip",
                &["link", "set", "dev", name, "up"],
                "ip link set up",
            )?;
            Ok(TunDevice {
                name: name.to_string(),
                fd,
                mtu,
            })
        }

        #[cfg(target_os = "windows")]
        {
            let _ = (name, mtu);
            Err(
                "Windows TUN (wintun.dll) is not bundled in this build. \
                 Server mode is currently macOS/Linux only."
                    .to_string(),
            )
        }
    }

    /// Configure an IPv4 (and optionally IPv6) address on the TUN device.
    /// Accepts CIDR notation, e.g. `10.88.0.1/24` or `fd00::1/64`. Multiple
    /// addresses can be passed comma-separated.
    pub fn configure_address(&self, addr_with_cidr: &str) -> Result<(), String> {
        for raw in addr_with_cidr.split(',') {
            let addr = raw.trim();
            if addr.is_empty() {
                continue;
            }
            self.configure_one_address(addr)?;
        }
        Ok(())
    }

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
            // Best-effort flush of stale addresses for the same family.
            let _ = run_ok(
                "ip",
                &[
                    if is_ipv6 { "-6" } else { "-4" },
                    "addr",
                    "flush",
                    "dev",
                    &self.name,
                ],
                "ip addr flush",
            );
            run_required(
                "ip",
                &["addr", "add", &cidr, "dev", &self.name],
                "ip addr add",
            )
        }

        #[cfg(target_os = "windows")]
        {
            let _ = (ip_only, prefix, is_ipv6);
            Err("Windows TUN address configuration not yet implemented".to_string())
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
            let _ = mtu;
            Err("Windows TUN set_mtu not yet implemented".to_string())
        }
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

        #[cfg(any(target_os = "linux", target_os = "macos"))]
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

// ---------------------------------------------------------------------------
// Linux implementation: native /dev/net/tun + TUNSETIFF ioctl.
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux {
    use libc::{c_int, c_short, c_ulong, ioctl, open, O_NONBLOCK, O_RDWR};
    use std::ffi::CString;

    // From <linux/if.h> / <linux/if_tun.h>.
    const IFNAMSIZ: usize = 16;
    const IFF_TUN: c_short = 0x0001;
    const IFF_NO_PI: c_short = 0x1000;
    // _IOW('T', 202, int) on most architectures. 0x400454ca is the canonical value.
    const TUNSETIFF: c_ulong = 0x400454ca;

    #[repr(C)]
    struct IfReq {
        ifr_name: [u8; IFNAMSIZ],
        ifr_flags: c_short,
        _padding: [u8; 22],
    }

    pub(crate) fn open_tun(name: &str) -> Result<i32, String> {
        if name.as_bytes().len() >= IFNAMSIZ {
            return Err(format!(
                "Interface name '{}' exceeds IFNAMSIZ ({} bytes)",
                name,
                IFNAMSIZ - 1
            ));
        }

        let path = CString::new("/dev/net/tun")
            .map_err(|e| format!("invalid tun path: {}", e))?;
        let fd = unsafe { open(path.as_ptr(), O_RDWR | O_NONBLOCK) };
        if fd < 0 {
            let err = std::io::Error::last_os_error();
            return Err(format!(
                "open(/dev/net/tun) failed: {} (need root + tun module loaded)",
                err
            ));
        }

        let mut req = IfReq {
            ifr_name: [0; IFNAMSIZ],
            ifr_flags: IFF_TUN | IFF_NO_PI,
            _padding: [0; 22],
        };
        for (dst, src) in req.ifr_name.iter_mut().zip(name.as_bytes()) {
            *dst = *src;
        }

        let rc = unsafe { ioctl(fd, TUNSETIFF as _, &mut req as *mut _ as *mut c_int) };
        if rc < 0 {
            let err = std::io::Error::last_os_error();
            unsafe { libc::close(fd) };
            return Err(format!("ioctl(TUNSETIFF) failed: {}", err));
        }

        Ok(fd)
    }
}
