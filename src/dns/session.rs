use crate::dns::backend::DnsBackend;
use crate::dns::error::DnsError;
use crate::dns::model::DnsSnapshot;

/// Tracks a single pending restore: one in-flight override at a time.
#[derive(Debug, Default)]
pub struct DnsState {
    pending: Option<DnsSnapshot>,
}

impl DnsState {
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// `true` if a snapshot is waiting to be applied by [`restore`](crate::dns::restore).
    pub fn has_pending_restore(&self) -> bool {
        self.pending.is_some()
    }

    /// Store snapshot for later restore (typically only set by override helpers).
    pub fn set_pending(&mut self, s: DnsSnapshot) {
        self.pending = Some(s);
    }

    /// Take and remove the pending snapshot without applying it.
    pub fn take_pending(&mut self) -> Option<DnsSnapshot> {
        self.pending.take()
    }

    pub fn clear(&mut self) {
        self.pending = None;
    }
}

/// Capture current DNS, store in `state`, then set `new_servers`. Fails if a restore is still pending.
pub fn begin_override_with<B: DnsBackend>(
    backend: &B,
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
    let snap = DnsSnapshot::capture(backend, service)?;
    backend.set_dns_manual(service, new_servers)?;
    state.set_pending(snap);
    Ok(())
}

/// Apply the stored [`DnsSnapshot`] and clear it. If apply fails, the snapshot is put back.
pub fn restore_with<B: DnsBackend>(backend: &B, state: &mut DnsState) -> Result<(), DnsError> {
    let Some(snap) = state.take_pending() else {
        return Err(DnsError::NothingToRestore);
    };
    match snap.apply(backend) {
        Ok(()) => Ok(()),
        Err(e) => {
            state.set_pending(snap);
            Err(e)
        }
    }
}
