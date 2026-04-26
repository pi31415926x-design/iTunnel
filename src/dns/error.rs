use std::fmt;

/// DNS helper failures (I/O, parse, platform, or session rules).
#[derive(Debug)]
pub enum DnsError {
    Io(std::io::Error),
    CommandFailed { cmd: String, code: Option<i32>, stderr: String },
    Parse(String),
    UnsupportedPlatform,
    /// `begin_override` was called while a restore snapshot was still held.
    ActiveSession,
    /// `restore` when no snapshot is stored.
    NothingToRestore,
    /// Empty server list for a manual set (invalid).
    EmptyServers,
}

impl fmt::Display for DnsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DnsError::Io(e) => write!(f, "{}", e),
            DnsError::CommandFailed { cmd, code, stderr } => write!(
                f,
                "command failed: {cmd} (status {:?}): {stderr}",
                code,
            ),
            DnsError::Parse(s) => write!(f, "parse: {s}"),
            DnsError::UnsupportedPlatform => write!(f, "DNS: unsupported on this platform"),
            DnsError::ActiveSession => write!(f, "DNS: an override is already active; restore first"),
            DnsError::NothingToRestore => write!(f, "DNS: nothing to restore"),
            DnsError::EmptyServers => write!(f, "DNS: at least one server address is required"),
        }
    }
}

impl std::error::Error for DnsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DnsError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DnsError {
    fn from(e: std::io::Error) -> Self {
        DnsError::Io(e)
    }
}
