use crate::dns::backend::DnsBackend;
use crate::dns::error::DnsError;

/// Immutable record of one network service’s DNS as last observed (for restore).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsSnapshot {
    pub service: String,
    pub mode: DnsMode,
}

/// Previous or target DNS mode for a service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsMode {
    /// DHCP / automatic (no fixed servers).
    Automatic,
    /// Fixed IPv4 DNS server addresses.
    Manual(Vec<String>),
}

impl DnsSnapshot {
    pub fn new(service: impl Into<String>, mode: DnsMode) -> Self {
        Self {
            service: service.into(),
            mode,
        }
    }

    /// Read current DNS via `backend` and package it for later restore.
    pub fn capture<B: DnsBackend>(backend: &B, service: &str) -> Result<Self, DnsError> {
        backend.capture_snapshot(service)
    }

    /// Re-apply this snapshot (restore after override, or copy settings).
    pub fn apply<B: DnsBackend>(&self, backend: &B) -> Result<(), DnsError> {
        backend.apply_snapshot(self)
    }
}
