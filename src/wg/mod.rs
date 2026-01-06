//我们将代码分为两部分：ffi (原始接口) 和 safe (安全封装)。
use libc::{c_char, c_int, c_longlong, c_void};
use std::ffi::{CStr, CString};
use std::ptr;
pub mod config;
pub mod store;

// ============================================================================
// 1. FFI 模块：对应 Go 导出的函数, 参考来源: api-apple.go
// ============================================================================
mod ffi {
    use super::*;
    // 定义 Logger 回调函数的类型签名
    // 对应 Go 中的 static void callLogger(...)
    pub type LoggerCallback = extern "C" fn(ctx: *mut c_void, level: c_int, msg: *const c_char);

    #[link(name = "wg-go")] // 假设编译出的库名为 libwg-go.so / libwg-go.a
    extern "C" {
        // func wgSetLogger(context, loggerFn uintptr)
        pub fn wgSetLogger(context: *mut c_void, logger_fn: LoggerCallback);

        // func wgTurnOn(settings *C.char, tunFd int32) int32
        pub fn wgTurnOn(settings: *const c_char, tun_fd: c_int) -> c_int;

        // func wgTurnOff(tunnelHandle int32)
        pub fn wgTurnOff(tunnel_handle: c_int);

        // func wgSetConfig(tunnelHandle int32, settings *C.char) int64
        pub fn wgSetConfig(tunnel_handle: c_int, settings: *const c_char) -> c_longlong;

        // func wgGetConfig(tunnelHandle int32) *C.char
        // 注意：返回值需要用 libc::free 释放
        pub fn wgGetConfig(tunnel_handle: c_int) -> *mut c_char;

        // func wgBumpSockets(tunnelHandle int32)
        pub fn wgBumpSockets(tunnel_handle: c_int);

        // func wgVersion() *C.char
        // 注意：返回值需要用 libc::free 释放
        pub fn wgVersion() -> *mut c_char;

        // func createTun(name *C.char, mtu int32) int32
        pub fn createTun(name: *const c_char, mtu: c_int) -> c_int;

        // func get wireguard tx and rx bytes
        pub fn wgGetStats(handle: i32, rx: *mut u64, tx: *mut u64);

        // func decodeConfig(encryptConfStr *C.char) *C.char
        // 注意：返回值需要用 libc::free 释放
        pub fn decodeConfig(encrypt_conf_str: *const c_char) -> *mut c_char;
    }
}

// ============================================================================
// 2. Safe Wrapper：提供符合 Rust 习惯的 API
// ============================================================================

pub struct WireGuardApi;

/// 日志级别枚举
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Verbose = 0,
    Error = 1,
    Unknown,
}

impl From<c_int> for LogLevel {
    fn from(val: c_int) -> Self {
        match val {
            0 => LogLevel::Verbose,
            1 => LogLevel::Error,
            _ => LogLevel::Unknown,
        }
    }
}

/// 全局的 Rust Logger 回调适配器
/// 这个函数会被 C 代码调用
extern "C" fn rust_logger_trampoline(_ctx: *mut c_void, level: c_int, msg: *const c_char) {
    let rust_level = LogLevel::from(level);
    let message = unsafe {
        if msg.is_null() {
            String::from("<null>")
        } else {
            CStr::from_ptr(msg).to_string_lossy().into_owned()
        }
    };

    // 接入 log crate
    match rust_level {
        LogLevel::Verbose => log::debug!("[WG-Go] {}", message),
        LogLevel::Error => log::error!("[WG-Go] {}", message),
        LogLevel::Unknown => log::warn!("[WG-Go] [Unknown Level] {}", message),
    }
}

impl WireGuardApi {
    /// 设置日志记录器
    pub fn set_logger() {
        unsafe {
            // 这里我们传 null 作为 context，如果需要传递 Rust 对象，需要更复杂的 Box::into_raw 处理
            ffi::wgSetLogger(ptr::null_mut(), rust_logger_trampoline);
        }
    }

    /// 获取版本号
    pub fn version() -> String {
        unsafe {
            let ptr = ffi::wgVersion();
            if ptr.is_null() {
                return "unknown".to_string();
            }
            let res = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            libc::free(ptr as *mut c_void); // 必须释放 Go 分配的内存
            res
        }
    }

    /// 创建 TUN 设备
    pub fn create_tun(name: &str, mtu: i32) -> Result<i32, String> {
        let c_name = CString::new(name).map_err(|_| "Invalid interface name")?;
        let fd = unsafe { ffi::createTun(c_name.as_ptr(), mtu as c_int) };
        if fd < 0 {
            Err("Failed to create TUN device".to_string())
        } else {
            Ok(fd)
        }
    }

    /// 开启 WireGuard 隧道
    pub fn turn_on(settings: &str, tun_fd: i32) -> Result<i32, String> {
        let c_settings = CString::new(settings).map_err(|_| "Invalid settings string")?;
        let handle = unsafe { ffi::wgTurnOn(c_settings.as_ptr(), tun_fd as c_int) };
        if handle < 0 {
            Err("Failed to turn on wireguard interface".to_string())
        } else {
            Ok(handle)
        }
    }

    /// 关闭隧道
    pub fn turn_off(handle: i32) {
        unsafe { ffi::wgTurnOff(handle as c_int) };
    }

    /// 更新配置
    pub fn set_config(handle: i32, settings: &str) -> Result<(), i64> {
        let c_settings = CString::new(settings).map_err(|_| -1i64)?;
        let ret = unsafe { ffi::wgSetConfig(handle as c_int, c_settings.as_ptr()) };
        if ret == 0 {
            Ok(())
        } else {
            Err(ret)
        }
    }

    /// 获取当前配置
    pub fn get_config(handle: i32) -> Option<String> {
        unsafe {
            let ptr = ffi::wgGetConfig(handle as c_int);
            if ptr.is_null() {
                return None;
            }
            let res = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            libc::free(ptr as *mut c_void); // 必须释放 Go 分配的内存
            Some(res)
        }
    }

    /// 解密配置
    pub fn decode_config(encrypt_conf: &str) -> String {
        use aes::cipher::{AsyncStreamCipher, KeyIvInit};
        use base64::{engine::general_purpose, Engine as _};
        type Aes128CfbDec = cfb_mode::Decryptor<aes::Aes128>;
        let key = [0x42; 16];
        let iv = [0x24; 16];
        let encrypted_bytes = match general_purpose::STANDARD.decode(encrypt_conf) {
            Ok(b) => b,
            Err(e) => {
                log::error!("Failed to decode base64 config: {}", e);
                return "{}".to_string();
            }
        };
        let mut buf = encrypted_bytes;
        Aes128CfbDec::new(&key.into(), &iv.into()).decrypt(&mut buf);
        String::from_utf8(buf).unwrap_or_else(|e| {
            log::error!("Failed to parse decrypted config as UTF-8: {}", e);
            "{}".to_string()
        })
    }

    pub fn bump_sockets(handle: i32) {
        unsafe { ffi::wgBumpSockets(handle as c_int) };
    }

    pub fn base64_to_hex(input: &str) -> Result<String, String> {
        use base64::{engine::general_purpose, Engine as _};
        let bytes = general_purpose::STANDARD
            .decode(input)
            .map_err(|e| format!("Failed to decode base64: {}", e))?;
        Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
    }

    /// 获取 WireGuard 接口的统计信息 (RX, TX)
    pub fn get_stats(handle: i32) -> Result<(u64, u64), String> {
        let mut rx: u64 = 0;
        let mut tx: u64 = 0;
        unsafe {
            ffi::wgGetStats(handle, &mut rx as *mut u64, &mut tx as *mut u64);
            //dump wireguard config
            //debug!("WireGuard Config is: {:?}", Self::get_config(handle));
        }
        Ok((rx, tx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_log() {
        crate::logging::init();
        WireGuardApi::set_logger();
        println!("Logger has been set. If any FFI calls log, they should appear in stdout.");
    }

    #[test]
    fn test_create_tun() {
        crate::logging::init();
        // 注意：在 macOS 上创建 TUN 设备通常需要 root 权限。
        // 如果没有 root 权限，此测试可能会失败。
        let tun_name = "utun233";
        match WireGuardApi::create_tun(tun_name, 1420) {
            Ok(fd) => {
                println!("Successfully created TUN device '{}', fd: {}", tun_name, fd);
                // 简单清理，虽然测试结束也会关闭
                unsafe { libc::close(fd) };
            }
            Err(e) => {
                eprintln!("Failed to create TUN device '{}': {}", tun_name, e);
                // 这里不 panic，因为在非 root 环境下失败是预期的
            }
        }
    }

    #[test]
    fn test_wg_turn_on() {
        crate::logging::init();
        // 此测试会一直运行，直到用户按 Ctrl+C
        // 请使用 `cargo test -- --nocapture` 运行以查看输出

        println!("Starting wg_turn_on test...");
        WireGuardApi::set_logger();

        let tun_name = "utun9981";
        let tun_fd = match WireGuardApi::create_tun(tun_name, 1420) {
            Ok(fd) => fd,
            Err(e) => {
                eprintln!("Failed to create TUN device: {}. Skipping turn_on test.", e);
                return;
            }
        };
        println!("TUN device created: fd={}", tun_fd);

        // 一个最小的有效配置示例 (需要有效的私钥格式)
        // 这里使用一个全零的 32 字节私钥的 Base64 编码
        let private_key_b64 = "yMlj3LbVKMW69kXXh0OpbfZUlEVmkYDao3bk6jTl/EQ=";
        let public_key_b64 = "qHaIfS7u47/U1AuigBDhOv/p/t6Gy+XKSUdYnPIEKDA=";
        let preshared_key_b64 = "FbXfpBRYbCoUAADjfwOMmjSQ8XB8ZcnjfsTN877JJcI=";

        // Convert to Hex
        let private_key =
            WireGuardApi::base64_to_hex(private_key_b64).expect("Failed to decode private key");
        let public_key =
            WireGuardApi::base64_to_hex(public_key_b64).expect("Failed to decode public key");
        let preshared_key =
            WireGuardApi::base64_to_hex(preshared_key_b64).expect("Failed to decode preshared key");
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
        println!("Wireguard conf: {}", settings);
        match WireGuardApi::turn_on(&settings, tun_fd) {
            Ok(handle) => {
                println!("WireGuard turned on successfully. Handle: {}", handle);
                println!("Tunnel is active. Press Ctrl+C to stop...");

                // 模拟长时间运行，直到 Ctrl+C
                loop {
                    thread::sleep(Duration::from_secs(1));
                }
            }
            Err(e) => {
                eprintln!("Failed to turn on WireGuard: {}", e);
                unsafe { libc::close(tun_fd) };
            }
        }
    }
}
