//! Local DNS: small primitives per platform, optional [`DnsState`] for one pending restore.
//!
//! Typical flow: [`begin_override`], work, then [`restore`]. You can also call
//! [`capture_snapshot`] + [`set_dns_manual`] / [`set_dns_automatic`] + [`apply_snapshot`]
//! for custom control.

mod error;
#[cfg(target_os = "macos")]
mod macos;
mod snapshot;
mod state;
#[cfg(not(target_os = "macos"))]
mod stub;

pub use error::DnsError;
pub use snapshot::{DnsMode, DnsSnapshot};
pub use state::DnsState;

#[cfg(target_os = "macos")]
use macos as platform;
#[cfg(not(target_os = "macos"))]
use stub as platform;

// ——— composable one-shot API ———

/// Read current mode for a network service (e.g. `Wi-Fi` on macOS).
pub fn get_dns_mode(service: &str) -> Result<DnsMode, DnsError> {
    platform::get_dns_mode(service)
}

/// Set fixed DNS servers for a service. Fails on empty `servers` (use [`set_dns_automatic`]).
pub fn set_dns_manual(service: &str, servers: &[String]) -> Result<(), DnsError> {
    platform::set_dns_manual(service, servers)
}

/// Clear manual DNS; use system/DHCP for that service.
pub fn set_dns_automatic(service: &str) -> Result<(), DnsError> {
    platform::set_dns_automatic(service)
}

/// List network service names available to `networksetup` (macOS) or the platform equivalent.
pub fn list_network_service_names() -> Result<Vec<String>, DnsError> {
    platform::list_network_service_names()
}

/// Read current state so it can be re-applied later.
pub fn capture_snapshot(service: &str) -> Result<DnsSnapshot, DnsError> {
    platform::capture_snapshot(service)
}

/// Re-apply a [`DnsSnapshot`] (restore after override, or copy settings).
pub fn apply_snapshot(snap: &DnsSnapshot) -> Result<(), DnsError> {
    platform::apply_snapshot(snap)
}

// ——— stateful: one active override, restore to snapshot ———

/// Capture current DNS, store in `state`, then set `new_servers`. Fails if a restore is still pending.
pub fn begin_override(
    state: &mut DnsState,
    service: &str,
    new_servers: &[String],
) -> Result<(), DnsError> {
    if state.has_pending_restore() {
        return Err(DnsError::ActiveSession);
    }
    if new_servers.is_empty() {
        return Err(DnsError::EmptyServers);
    }
    let snap = capture_snapshot(service)?;
    set_dns_manual(service, new_servers)?;
    state.set_pending(snap);
    Ok(())
}

/// Apply the stored [`DnsSnapshot`] and clear it. If apply fails, the snapshot is put back.
pub fn restore(state: &mut DnsState) -> Result<(), DnsError> {
    let Some(snap) = state.take_pending() else {
        return Err(DnsError::NothingToRestore);
    };
    match apply_snapshot(&snap) {
        Ok(()) => Ok(()),
        Err(e) => {
            state.set_pending(snap);
            Err(e)
        }
    }
}

// macOS-only: expose raw `networksetup` escape hatch for power users
#[cfg(target_os = "macos")]
pub mod macos_raw {
    pub use crate::dns::macos::run_networksetup;
}
