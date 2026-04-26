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
}
