//! Non-macOS: all operations return [`DnsError::UnsupportedPlatform`].
#![allow(dead_code)]

use crate::dns::backend::DnsBackend;
use crate::dns::error::DnsError;
use crate::dns::model::DnsMode;

#[derive(Debug, Clone, Copy, Default)]
pub struct Unsupported;

impl DnsBackend for Unsupported {
    fn get_dns_mode(&self, _service: &str) -> Result<DnsMode, DnsError> {
        Err(DnsError::UnsupportedPlatform)
    }

    fn set_dns_automatic(&self, _service: &str) -> Result<(), DnsError> {
        Err(DnsError::UnsupportedPlatform)
    }

    fn set_dns_manual(&self, _service: &str, _servers: &[String]) -> Result<(), DnsError> {
        Err(DnsError::UnsupportedPlatform)
    }

    fn list_network_service_names(&self) -> Result<Vec<String>, DnsError> {
        Err(DnsError::UnsupportedPlatform)
    }
}
