use crate::dns::snapshot::DnsSnapshot;

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

    /// Store snapshot for later restore (typically only set by the crate’s override helpers).
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
