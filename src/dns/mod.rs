//! Local DNS: **data** ([`DnsMode`], [`DnsSnapshot`]), **pure interpretation** ([`parse`]),
//! **effects** ([`DnsBackend`]), and optional **session** state ([`DnsState`]).
//!
//! Typical flow: [`begin_override`], work, then [`restore`]. For custom control, use
//! [`DnsSnapshot::capture`] / [`DnsBackend`] primitives / [`DnsSnapshot::apply`].

mod backend;
mod error;
mod model;
pub mod parse;
mod session;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(not(target_os = "macos"))]
mod stub;

pub use backend::DnsBackend;
pub use error::DnsError;
pub use model::{DnsMode, DnsSnapshot};
pub use session::{begin_override_with, restore_with, DnsState};

#[cfg(target_os = "macos")]
pub use macos::NetworkSetup;
#[cfg(not(target_os = "macos"))]
pub use stub::Unsupported;

#[cfg(target_os = "macos")]
const DEFAULT: NetworkSetup = NetworkSetup;
#[cfg(not(target_os = "macos"))]
const DEFAULT: Unsupported = Unsupported;

/// Read current mode for a network service (e.g. `Wi-Fi` on macOS).
pub fn get_dns_mode(service: &str) -> Result<DnsMode, DnsError> {
    DEFAULT.get_dns_mode(service)
}

/// Set fixed DNS servers for a service. Fails on empty `servers` (use [`set_dns_automatic`]).
pub fn set_dns_manual(service: &str, servers: &[String]) -> Result<(), DnsError> {
    DEFAULT.set_dns_manual(service, servers)
}

/// Clear manual DNS; use system/DHCP for that service.
pub fn set_dns_automatic(service: &str) -> Result<(), DnsError> {
    DEFAULT.set_dns_automatic(service)
}

/// List network service names available to `networksetup` (macOS) or the platform equivalent.
pub fn list_network_service_names() -> Result<Vec<String>, DnsError> {
    DEFAULT.list_network_service_names()
}

/// Read current state so it can be re-applied later.
pub fn capture_snapshot(service: &str) -> Result<DnsSnapshot, DnsError> {
    DEFAULT.capture_snapshot(service)
}

/// Re-apply a [`DnsSnapshot`] (restore after override, or copy settings).
pub fn apply_snapshot(snap: &DnsSnapshot) -> Result<(), DnsError> {
    DEFAULT.apply_snapshot(snap)
}

/// Capture current DNS, store in `state`, then set `new_servers`. Fails if a restore is still pending.
pub fn begin_override(
    state: &mut DnsState,
    service: &str,
    new_servers: &[String],
) -> Result<(), DnsError> {
    begin_override_with(&DEFAULT, state, service, new_servers)
}

/// Apply the stored [`DnsSnapshot`] and clear it. If apply fails, the snapshot is put back.
pub fn restore(state: &mut DnsState) -> Result<(), DnsError> {
    restore_with(&DEFAULT, state)
}

#[cfg(target_os = "macos")]
pub mod macos_raw {
    pub use crate::dns::macos::run_networksetup;
}
