use crate::dns::error::DnsError;
use crate::dns::snapshot::{DnsMode, DnsSnapshot};
use log::debug;
use std::process::Command;

const LINE_ANY_NOT_SET: &str = "There aren't any DNS servers set on";

/// Run `networksetup` and return trimmed stdout, or an error.
pub fn run_networksetup(args: &[&str]) -> Result<String, DnsError> {
    let out = command_output("networksetup", args)?;
    Ok(out)
}

/// List human-readable service names (excludes the header `*` line).
pub fn list_network_service_names() -> Result<Vec<String>, DnsError> {
    let s = run_networksetup(&["-listallnetworkservices"])?;
    let mut names = Vec::new();
    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        names.push(line.to_string());
    }
    Ok(names)
}

/// `networksetup -getdnsservers <service>` as [`DnsMode`].
pub fn get_dns_mode(service: &str) -> Result<DnsMode, DnsError> {
    let s = run_networksetup(&["-getdnsservers", service])?;
    parse_getdnsservers(&s)
}

fn parse_getdnsservers(stdout: &str) -> Result<DnsMode, DnsError> {
    if stdout
        .lines()
        .any(|l| l.contains(LINE_ANY_NOT_SET) || l.trim() == "empty")
    {
        return Ok(DnsMode::Automatic);
    }
    let addrs: Vec<String> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| {
            !l.is_empty() && !l.contains(LINE_ANY_NOT_SET) && *l != "empty"
        })
        .map(String::from)
        .collect();
    if addrs.is_empty() {
        Ok(DnsMode::Automatic)
    } else {
        Ok(DnsMode::Manual(addrs))
    }
}

/// Point DNS at the given servers (replaces any previous manual list for that service).
pub fn set_dns_manual(service: &str, servers: &[String]) -> Result<(), DnsError> {
    if servers.is_empty() {
        return Err(DnsError::EmptyServers);
    }
    let mut args: Vec<String> = vec!["-setdnsservers".into(), service.into()];
    args.extend(servers.iter().cloned());
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    run_networksetup_status(&args_ref)
}

/// Restore DHCP/“no fixed DNS” for that service.
pub fn set_dns_automatic(service: &str) -> Result<(), DnsError> {
    run_networksetup_status(&["-setdnsservers", service, "empty"])
}

/// Current DNS for a service, packaged for restore.
pub fn capture_snapshot(service: &str) -> Result<DnsSnapshot, DnsError> {
    let mode = get_dns_mode(service)?;
    Ok(DnsSnapshot::new(service, mode))
}

/// Apply a previously captured state (use after an override, or to duplicate settings).
pub fn apply_snapshot(snap: &DnsSnapshot) -> Result<(), DnsError> {
    match &snap.mode {
        DnsMode::Automatic => set_dns_automatic(&snap.service),
        DnsMode::Manual(servers) => set_dns_manual(&snap.service, servers),
    }
}

// --- small IO helpers (single responsibility) ---

fn command_output(cmd: &str, args: &[&str]) -> Result<String, DnsError> {
    debug!("dns: {cmd} {:?}", args);
    let output = Command::new(cmd).args(args).output()?;
    if !output.status.success() {
        return Err(DnsError::CommandFailed {
            cmd: format!("{} {:?}", cmd, args),
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_networksetup_status(args: &[&str]) -> Result<(), DnsError> {
    let _ = command_output("networksetup", args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_getdnsservers;
    use crate::dns::snapshot::DnsMode;

    #[test]
    fn parse_automatic_english() {
        let t = "There aren't any DNS servers set on Wi-Fi.\n";
        let m = parse_getdnsservers(t).unwrap();
        assert_eq!(m, DnsMode::Automatic);
    }

    #[test]
    fn parse_manual_one() {
        let t = "8.8.8.8\n";
        let m = parse_getdnsservers(t).unwrap();
        assert!(matches!(m, DnsMode::Manual(v) if v == vec!["8.8.8.8"]));
    }
}
