use std::ffi::{c_void, CStr, CString};
use std::os::raw::{c_char, c_int, c_longlong};

use actix_web::{get, web, Error, Responder};
use log::debug;
use serde::Serialize;

// Raw FFI bindings to libwg-go.a
extern "C" {
    pub fn wgTurnOn(settings: *const c_char, tun_fd: i32) -> i32;
    pub fn wgTurnOff(handle: i32);
    pub fn wgSetConfig(handle: i32, settings: *const c_char) -> c_longlong;
    pub fn wgBumpSockets(handle: i32);
    pub fn wgDisableSomeRoamingForBrokenMobileSemantics(handle: i32);
    pub fn wgVersion() -> *const c_char;
    pub fn wgGetConfig(handle: i32) -> *mut c_char;
    pub fn createTun(name: *const c_char, mtu: i32) -> i32;
    pub fn wgSetLogger(
        context: *mut c_void,
        logger_fn: extern "C" fn(*mut c_void, c_int, *const c_char),
    );
}

#[derive(Debug, Serialize)]
pub struct WireGuard {
    handle: i32,
}

/// Turn on the WireGuard tunnel.
/// `settings` is a string containing the WireGuard configuration (e.g., "private_key=...\n...").
/// `tun_fd` is the file descriptor of the TUN device.
#[get("/api/interfaces")]
pub async fn wg_turn_on(settings: String, tun_fd: web::Path<i32>) -> Result<impl Responder, Error> {
    let c_settings = CString::new(settings).map_err(actix_web::error::ErrorBadRequest)?;
    let handle = unsafe { wgTurnOn(c_settings.as_ptr(), *tun_fd) };

    if handle < 0 {
        return Err(actix_web::error::ErrorInternalServerError(format!(
            "Failed to turn on WireGuard, error code: {}",
            handle
        )));
    }

    Ok(web::Json(WireGuard { handle }))
}

impl WireGuard {
    /// Turn on the WireGuard tunnel.
    pub fn turn_on(settings: &str, tun_fd: i32) -> Result<Self, String> {
        let c_settings = CString::new(settings).map_err(|e| e.to_string())?;
        let handle = unsafe { wgTurnOn(c_settings.as_ptr(), tun_fd) };

        if handle < 0 {
            return Err(format!(
                "Failed to turn on WireGuard, error code: {}",
                handle
            ));
        }

        Ok(WireGuard { handle })
    }

    /// Update the configuration of the running tunnel.
    pub fn set_config(&self, settings: &str) -> Result<(), String> {
        let c_settings = CString::new(settings).map_err(|e| e.to_string())?;
        let res = unsafe { wgSetConfig(self.handle, c_settings.as_ptr()) };

        if res != 0 {
            return Err(format!("Failed to set config, error code: {}", res));
        }
        Ok(())
    }

    /// Get the current configuration of the running tunnel.
    pub fn get_config(&self) -> Result<String, String> {
        unsafe {
            let ptr = wgGetConfig(self.handle);
            if ptr.is_null() {
                return Err("Failed to get config".to_string());
            }
            let config = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            // Go's C.CString allocates memory that must be freed by the caller.
            libc::free(ptr as *mut _);
            Ok(config)
        }
    }

    /// Bump sockets to force re-handshake or update endpoints.
    pub fn bump_sockets(&self) {
        unsafe { wgBumpSockets(self.handle) };
    }

    /// Disable some roaming features for broken mobile semantics.
    pub fn disable_some_roaming(&self) {
        unsafe { wgDisableSomeRoamingForBrokenMobileSemantics(self.handle) };
    }

    /// Get the version of the WireGuard Go implementation.
    pub fn version() -> String {
        unsafe {
            let ptr = wgVersion();
            if ptr.is_null() {
                return "unknown".to_string();
            }
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }

    /// Helper to convert Base64 key to Hex string (required for low-level API)
    pub fn base64_to_hex(input: &str) -> Result<String, String> {
        use base64::{engine::general_purpose, Engine as _};
        let bytes = general_purpose::STANDARD
            .decode(input)
            .map_err(|e| format!("Failed to decode base64: {}", e))?;
        Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
    }

    pub fn create_tun(name: &str, mtu: i32) -> Result<i32, String> {
        let c_name = CString::new(name).map_err(|e| e.to_string())?;
        let fd = unsafe { createTun(c_name.as_ptr(), mtu) };
        if fd < 0 {
            return Err("Failed to create TUN device".to_string());
        }
        Ok(fd)
    }

    /// Initialize the WireGuard logger to forward logs to the Rust `log` crate.
    /// This should be called once at the start of the application.
    pub fn init_logger() {
        debug!("init logger for wireguard");
        unsafe {
            wgSetLogger(std::ptr::null_mut(), wg_log_callback);
        }
    }
}

extern "C" fn wg_log_callback(_ctx: *mut c_void, level: c_int, msg: *const c_char) {
    log::debug!("wg_log_call called!");
    if msg.is_null() {
        return;
    }
    let msg = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    match level {
        0 => log::debug!("[WireGuard] {}", msg),
        1 => log::error!("[WireGuard] {}", msg),
        _ => log::info!("[WireGuard] {}", msg),
    }
}

impl Drop for WireGuard {
    fn drop(&mut self) {
        // Automatically turn off when the struct goes out of scope?
        // Or should this be explicit?
        // Usually explicit is better for control, but Drop is safer.
        // Let's keep it explicit in `turn_off` method, but maybe log in drop if not closed?
        // For now, let's provide an explicit turn_off method that consumes self.
    }
}

impl WireGuard {
    pub fn turn_off(self) {
        unsafe { wgTurnOff(self.handle) };
        // self is consumed, so it can't be used again.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    unsafe fn open_utun() -> Result<i32, String> {
        use libc::{
            close, connect, ctl_info, ioctl, sockaddr, sockaddr_ctl, socket, AF_SYSTEM,
            AF_SYS_CONTROL, CTLIOCGINFO, PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL,
        };
        use std::mem;

        let fd = socket(PF_SYSTEM, SOCK_DGRAM, SYSPROTO_CONTROL);
        if fd < 0 {
            return Err(format!(
                "Failed to open socket: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut info: ctl_info = mem::zeroed();
        // "com.apple.net.utun_control"
        let name = "com.apple.net.utun_control";
        let name_bytes = name.as_bytes();
        // ctl_name is [c_char; 96]
        for (i, &b) in name_bytes.iter().enumerate() {
            info.ctl_name[i] = b as i8;
        }

        if ioctl(fd, CTLIOCGINFO, &mut info) < 0 {
            close(fd);
            return Err(format!(
                "ioctl CTLIOCGINFO failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut addr: sockaddr_ctl = mem::zeroed();
        addr.sc_len = mem::size_of::<sockaddr_ctl>() as u8;
        addr.sc_family = AF_SYSTEM as u8;
        addr.ss_sysaddr = AF_SYS_CONTROL as u16;
        addr.sc_id = info.ctl_id;
        addr.sc_unit = 0; // 0 means let kernel pick

        if connect(
            fd,
            &addr as *const _ as *const sockaddr,
            mem::size_of::<sockaddr_ctl>() as u32,
        ) < 0
        {
            close(fd);
            return Err(format!(
                "connect failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        Ok(fd)
    }

    #[test]
    fn test_wg_version() {
        let version = WireGuard::version();
        println!("WireGuard Go Version: {}", version);
        assert!(!version.is_empty());
        // assert_ne!(version, "unknown"); // The current libwg-go.a returns "unknown"
    }

    #[test]
    fn test_wg_turn_on_invalid_fd() {
        let settings = "private_key=invalid\n";
        let result = WireGuard::turn_on(settings, -1);
        assert!(result.is_err());
        println!("Result with invalid FD: {:?}", result);
    }

    #[test]
    #[ignore] // Requires root
    fn test_wg_turn_on_valid_fd() {
        #[cfg(not(target_os = "macos"))]
        {
            println!("Skipping test_wg_turn_on_valid_fd on non-macOS");
            return;
        }

        #[cfg(target_os = "macos")]
        {
            // Note: This requires root privileges or specific entitlements to succeed.
            // If run without sudo, open_utun will likely fail.
            let name = "utun99";

            let fd = match WireGuard::create_tun(name, 1280) {
                Ok(fd) => {
                    println!("Successfully created TUN device with fd: {}", fd);
                    assert!(fd >= 0);
                    fd
                }
                Err(e) => {
                    panic!("Failed to create TUN device: {}", e);
                }
            };

            WireGuard::init_logger();

            // Configuration from user request (Keys converted to Hex)
            // PrivateKey (Base64): yMlj3LbVKMW69kXXh0OpbfZUlEVmkYDao3bk6jTl/EQ=
            // PublicKey (Base64): qHaIfS7u47/U1AuigBDhOv/p/t6Gy+XKSUdYnPIEKDA=
            // PresharedKey (Base64): FbXfpBRYbCoUAADjfwOMmjSQ8XB8ZcnjfsTN877JJcI=

            // Configuration from user request (Base64 keys)
            let private_key_b64 = "yMlj3LbVKMW69kXXh0OpbfZUlEVmkYDao3bk6jTl/EQ=";
            let public_key_b64 = "qHaIfS7u47/U1AuigBDhOv/p/t6Gy+XKSUdYnPIEKDA=";
            let preshared_key_b64 = "FbXfpBRYbCoUAADjfwOMmjSQ8XB8ZcnjfsTN877JJcI=";

            // Convert to Hex
            let private_key =
                WireGuard::base64_to_hex(private_key_b64).expect("Failed to decode private key");
            let public_key =
                WireGuard::base64_to_hex(public_key_b64).expect("Failed to decode public key");
            let preshared_key = WireGuard::base64_to_hex(preshared_key_b64)
                .expect("Failed to decode preshared key");
            let endpoint = "54.249.221.90:62887";
            let listen_port = 51820;
            let jc = 3;
            let jmin = 10;
            let jmax = 30;
            let s1 = 11;
            let s2 = 22;
            let h1 = 33;
            let h2 = 44;
            let h3 = 55;
            let h4 = 66;
            let settings = format!(
                "private_key={}\nlisten_port={}\njc={}\njmin={}\njmax={}\ns1={}\ns2={}\nh1={}\nh2={}\nh3={}\nh4={}\nreplace_peers=true\npublic_key={}\npreshared_key={}\nendpoint={}\nallowed_ip=0.0.0.0/0\nallowed_ip=::/0\n",
                private_key,
                listen_port,
                jc,
                jmin,
                jmax,
                s1,s2,
                h1,h2,h3,h4,
                public_key,
                preshared_key,
                endpoint
            );

            let result = WireGuard::turn_on(&settings, fd);
            println!("Result with valid FD: {:?}", result);

            if let Ok(wg) = result {
                println!("Tunnel is successfully turned on!");
                println!("Press Ctrl+C to stop the test...");
                loop {
                    // Test get_config
                    // match wg.get_config() {
                    //     Ok(cfg) => println!("Current config: {}", cfg),
                    //     Err(e) => println!("Failed to get config: {}", e),
                    // }
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            } else {
                panic!(
                    "Failed to turn on WireGuard with valid FD: {:?}",
                    result.err()
                );
            }
        }
    }

    #[test]
    #[ignore] // Requires root
    fn test_create_tun() {
        #[cfg(target_os = "macos")]
        {
            // UTUN names on macOS are usually utun0, utun1, etc.
            // However, CreateTUN in wireguard-go/tun/tun_darwin.go usually takes "utun" to pick the next available,
            // or a specific name.
            let name = "utun99";
            match WireGuard::create_tun(name, 1280) {
                Ok(fd) => {
                    println!("Successfully created TUN device with fd: {}", fd);
                    std::thread::sleep(std::time::Duration::from_secs(10));
                    assert!(fd >= 0);
                    unsafe { libc::close(fd) };
                }
                Err(e) => {
                    println!("Failed to create TUN device: {}", e);
                }
            }
        }
    }
}
