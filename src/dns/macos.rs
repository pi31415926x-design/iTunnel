use crate::dns::backend::DnsBackend;
use crate::dns::error::DnsError;
use crate::dns::model::DnsMode;
use crate::dns::parse::interpret_getdnsservers;
use log::debug;
use std::process::Command;

/// macOS backend: `networksetup` commands.
#[derive(Debug, Clone, Copy, Default)]
pub struct NetworkSetup;

/// Run `networksetup` and return trimmed stdout, or an error.
pub fn run_networksetup(args: &[&str]) -> Result<String, DnsError> {
    command_output("networksetup", args)
}

impl DnsBackend for NetworkSetup {
    fn get_dns_mode(&self, service: &str) -> Result<DnsMode, DnsError> {
        let s = run_networksetup(&["-getdnsservers", service])?;
        Ok(interpret_getdnsservers(&s))
    }

    fn set_dns_manual(&self, service: &str, servers: &[String]) -> Result<(), DnsError> {
        if servers.is_empty() {
            return Err(DnsError::EmptyServers);
        }
        let mut args: Vec<String> = vec!["-setdnsservers".into(), service.into()];
        args.extend(servers.iter().cloned());
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        run_networksetup_status(&args_ref)
    }

    fn set_dns_automatic(&self, service: &str) -> Result<(), DnsError> {
        run_networksetup_status(&["-setdnsservers", service, "empty"])
    }

    fn list_network_service_names(&self) -> Result<Vec<String>, DnsError> {
        let s = run_networksetup(&["-listallnetworkservices"])?;
        Ok(s.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('*'))
            .map(String::from)
            .collect())
    }
}

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
