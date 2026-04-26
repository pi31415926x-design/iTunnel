use crate::dns::error::DnsError;
use crate::dns::model::{DnsMode, DnsSnapshot};

/// Platform I/O boundary: observe and change DNS for one network service.
///
/// Implementations supply primitives; [`capture_snapshot`](DnsBackend::capture_snapshot) and
/// [`apply_snapshot`](DnsBackend::apply_snapshot) compose them by default.
pub trait DnsBackend {
    fn get_dns_mode(&self, service: &str) -> Result<DnsMode, DnsError>;
    fn set_dns_manual(&self, service: &str, servers: &[String]) -> Result<(), DnsError>;
    fn set_dns_automatic(&self, service: &str) -> Result<(), DnsError>;
    fn list_network_service_names(&self) -> Result<Vec<String>, DnsError>;

    fn capture_snapshot(&self, service: &str) -> Result<DnsSnapshot, DnsError> {
        let mode = self.get_dns_mode(service)?;
        Ok(DnsSnapshot::new(service, mode))
    }

    fn apply_snapshot(&self, snap: &DnsSnapshot) -> Result<(), DnsError> {
        match &snap.mode {
            DnsMode::Automatic => self.set_dns_automatic(&snap.service),
            DnsMode::Manual(servers) => self.set_dns_manual(&snap.service, servers),
        }
    }
}

impl<B: DnsBackend + ?Sized> DnsBackend for &B {
    fn get_dns_mode(&self, service: &str) -> Result<DnsMode, DnsError> {
        (**self).get_dns_mode(service)
    }

    fn set_dns_manual(&self, service: &str, servers: &[String]) -> Result<(), DnsError> {
        (**self).set_dns_manual(service, servers)
    }

    fn set_dns_automatic(&self, service: &str) -> Result<(), DnsError> {
        (**self).set_dns_automatic(service)
    }

    fn list_network_service_names(&self) -> Result<Vec<String>, DnsError> {
        (**self).list_network_service_names()
    }
}
