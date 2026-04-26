//! Placeholder implementation for non-macOS targets (returns [`DnsError::UnsupportedPlatform`]).
#![allow(dead_code)] // Not linked on macOS; some tools still analyze this file.

use crate::dns::error::DnsError;
use crate::dns::snapshot::{DnsMode, DnsSnapshot};

pub fn get_dns_mode(_service: &str) -> Result<DnsMode, DnsError> {
    Err(DnsError::UnsupportedPlatform)
}

pub fn set_dns_automatic(_service: &str) -> Result<(), DnsError> {
    Err(DnsError::UnsupportedPlatform)
}

pub fn set_dns_manual(_service: &str, _servers: &[String]) -> Result<(), DnsError> {
    Err(DnsError::UnsupportedPlatform)
}

pub fn list_network_service_names() -> Result<Vec<String>, DnsError> {
    Err(DnsError::UnsupportedPlatform)
}

pub fn capture_snapshot(_service: &str) -> Result<DnsSnapshot, DnsError> {
    Err(DnsError::UnsupportedPlatform)
}

pub fn apply_snapshot(_snap: &DnsSnapshot) -> Result<(), DnsError> {
    Err(DnsError::UnsupportedPlatform)
}
