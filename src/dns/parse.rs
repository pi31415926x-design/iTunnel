//! Pure interpreter: `networksetup -getdnsservers` stdout → [`DnsMode`](crate::dns::model::DnsMode).

use crate::dns::model::DnsMode;

const LINE_ANY_NOT_SET: &str = "There aren't any DNS servers set on";

/// Interpret `networksetup -getdnsservers` stdout (no I/O).
pub fn interpret_getdnsservers(stdout: &str) -> DnsMode {
    if stdout.lines().any(|l| l.contains(LINE_ANY_NOT_SET) || l.trim() == "empty") {
        return DnsMode::Automatic;
    }
    let addrs: Vec<String> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.contains(LINE_ANY_NOT_SET) && *l != "empty")
        .map(String::from)
        .collect();
    if addrs.is_empty() {
        DnsMode::Automatic
    } else {
        DnsMode::Manual(addrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dns::model::DnsMode;

    #[test]
    fn parse_automatic_english() {
        let t = "There aren't any DNS servers set on Wi-Fi.\n";
        assert_eq!(interpret_getdnsservers(t), DnsMode::Automatic);
    }

    #[test]
    fn parse_manual_one() {
        let t = "8.8.8.8\n";
        assert!(matches!(
            interpret_getdnsservers(t),
            DnsMode::Manual(v) if v == vec!["8.8.8.8"]
        ));
    }
}
