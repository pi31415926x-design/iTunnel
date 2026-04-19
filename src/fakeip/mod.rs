use libc::{c_char, c_int};
use std::ffi::{CStr, CString};
use std::ptr;

// FFI Declarations
extern "C" {
    pub fn rust_init(config: *const c_char);
    pub fn rust_cleanup();
    pub fn rust_set_logger(callback: extern "C" fn(*const c_char));
    pub fn rust_set_route_callback(callback: extern "C" fn(*const c_char));
    pub fn rust_set_global_mode(enabled: bool);
    pub fn rust_load_rules(path: *const c_char) -> c_int;
    pub fn rust_load_rules_str(content: *const c_char) -> c_int;
    pub fn rust_match_domain(domain: *const c_char) -> c_int;
    pub fn rust_get_fake_ip(domain: *const c_char) -> *mut c_char;
    pub fn rust_get_domain_by_ip(ip: *const c_char) -> *mut c_char;
    pub fn rust_free_string(s: *mut c_char);
    pub fn rust_start_dns_proxy(port: u16);
    pub fn rust_set_if_index(if_index: u32);
    pub fn rust_create_bridge() -> c_int;
    pub fn rust_start_bridge(tun_fd: c_int);
}

/// Safe wrapper for LibFakeIP
pub struct FakeIP;

impl FakeIP {
    /// Initialize libfakeip runtime
    pub fn init() {
        unsafe { rust_init(ptr::null()) };
    }

    /// Cleanup resources
    pub fn cleanup() {
        unsafe { rust_cleanup() };
    }

    /// Register log callback
    pub fn set_logger(callback: extern "C" fn(*const c_char)) {
        unsafe { rust_set_logger(callback) };
    }

    /// Register route callback
    pub fn set_route_callback(callback: extern "C" fn(*const c_char)) {
        unsafe { rust_set_route_callback(callback) };
    }

    /// Set global proxy mode
    pub fn set_global_mode(enabled: bool) {
        unsafe { rust_set_global_mode(enabled) };
    }

    /// Load rules from file
    pub fn load_rules(path: &str) -> bool {
        if let Ok(c_path) = CString::new(path) {
            unsafe { rust_load_rules(c_path.as_ptr()) == 0 }
        } else {
            false
        }
    }

    /// Load rules from string
    pub fn load_rules_str(content: &str) -> bool {
        if let Ok(c_content) = CString::new(content) {
            unsafe { rust_load_rules_str(c_content.as_ptr()) == 0 }
        } else {
            false
        }
    }

    /// Check if domain should be proxied
    pub fn match_domain(domain: &str) -> bool {
        if let Ok(c_domain) = CString::new(domain) {
            unsafe { rust_match_domain(c_domain.as_ptr()) == 1 }
        } else {
            false
        }
    }

    /// Get or allocate Fake IP for domain
    pub fn get_fake_ip(domain: &str) -> Option<String> {
        if let Ok(c_domain) = CString::new(domain) {
            unsafe {
                let ptr = rust_get_fake_ip(c_domain.as_ptr());
                if !ptr.is_null() {
                    let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
                    rust_free_string(ptr);
                    return Some(s);
                }
            }
        }
        None
    }

    /// Reverse lookup domain for a Fake IP
    pub fn get_domain_by_ip(ip: &str) -> Option<String> {
        if let Ok(c_ip) = CString::new(ip) {
            unsafe {
                let ptr = rust_get_domain_by_ip(c_ip.as_ptr());
                if !ptr.is_null() {
                    let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
                    rust_free_string(ptr);
                    return Some(s);
                }
            }
        }
        None
    }

    /// Start DNS proxy on specified port
    pub fn start_dns_proxy(port: u16) {
        unsafe { rust_start_dns_proxy(port) };
    }

    /// Set interface index for the bridge
    pub fn set_if_index(index: u32) {
        unsafe { rust_set_if_index(index) };
    }

    /// Create communication bridge and return file descriptor
    pub fn create_bridge() -> i32 {
        unsafe { rust_create_bridge() }
    }

    /// Start TUN bridge on given file descriptor
    pub fn start_bridge(tun_fd: i32) {
        unsafe { rust_start_bridge(tun_fd) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_cleanup() {
        FakeIP::init();
        FakeIP::cleanup();
    }

    #[test]
    fn test_fake_ip_allocation() {
        FakeIP::init();
        if let Some(ip) = FakeIP::get_fake_ip("google.com") {
            assert!(ip.starts_with("198.18") || ip.starts_with("198.19"));
            if let Some(domain) = FakeIP::get_domain_by_ip(&ip) {
                assert_eq!(domain, "google.com");
            }
        }
        FakeIP::cleanup();
    }
}
