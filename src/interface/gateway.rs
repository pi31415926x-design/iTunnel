use log::{debug, error, info};
#[cfg(not(target_os = "windows"))]
use std::process::Command;

#[cfg(target_os = "macos")]
const PF_CONF_PATH: &str = "/tmp/pf_vpn_gateway.conf";

pub fn enable_gateway(vpn_if: &str) -> std::io::Result<()> {
    info!("🚀 Enabling VPN Gateway mode...");

    // 1. Detect physical interface (LAN)
    let lan_if = get_default_interface()?;
    info!("📍 Detected physical interface (LAN): {}", lan_if);
    info!("🌐 Forwarding to VPN interface: {}", vpn_if);

    #[cfg(target_os = "macos")]
    {
        use std::fs;
        // Enable IP forwarding
        set_sysctl("net.inet.ip.forwarding", "1")?;
        set_sysctl("net.inet6.ip6.forwarding", "1")?;

        // Generate PF configuration
        let pf_rules = format!(
            "# IPv4 NAT\nnat on {} inet from {}:network to any -> ({})\n# IPv6 NAT\nnat on {} inet6 from {}:network to any -> ({})\n",
            vpn_if, lan_if, vpn_if, vpn_if, lan_if, vpn_if
        );
        fs::write(PF_CONF_PATH, pf_rules)?;

        // Load and enable PF
        run_command("pfctl", &["-F", "all"])?;
        run_command("pfctl", &["-e"])?;
        run_command("pfctl", &["-f", PF_CONF_PATH])?;
    }

    #[cfg(target_os = "linux")]
    {
        // Enable IP forwarding
        set_sysctl("net.ipv4.ip_forward", "1")?;

        // Add iptables MASQUERADE rule
        run_command(
            "iptables",
            &[
                "-t",
                "nat",
                "-A",
                "POSTROUTING",
                "-o",
                vpn_if,
                "-j",
                "MASQUERADE",
            ],
        )?;
    }

    #[cfg(target_os = "windows")]
    {
        // Enable forwarding on all interfaces (or specific ones if needed)
        let ps_cmd = format!("Get-NetIPInterface | Set-NetIPInterface -Forwarding Enabled");
        run_command("powershell", &["-Command", &ps_cmd])?;

        // Set up NAT (Requires a specific internal subnet, this is a simplified example)
        // In a real scenario, we might need to find the specific subnet of lan_if
        let ps_nat = format!("New-NetNat -Name 'VPN-Gateway-NAT' -InternalIPInterfaceAddressPrefix '192.168.137.0/24'");
        let _ = run_command("powershell", &["-Command", &ps_nat]);
    }

    let lan_ip = get_interface_ip(&lan_if)?;
    info!(
        "✅ Gateway mode enabled. Client gateway should be set to: {}",
        lan_ip
    );

    Ok(())
}

pub fn disable_gateway() -> std::io::Result<()> {
    info!("🛑 Disabling VPN Gateway mode...");

    #[cfg(target_os = "macos")]
    {
        use std::fs;
        // Disable IP forwarding
        set_sysctl("net.inet.ip.forwarding", "0")?;
        set_sysctl("net.inet6.ip6.forwarding", "0")?;

        // Revert PF configuration
        run_command("pfctl", &["-f", "/etc/pf.conf"])?;
        run_command("pfctl", &["-d"])?;

        // Cleanup
        if std::path::Path::new(PF_CONF_PATH).exists() {
            let _ = fs::remove_file(PF_CONF_PATH);
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Disable IP forwarding
        set_sysctl("net.ipv4.ip_forward", "0")?;

        // We can't easily remove the specific rule without knowing it,
        // but 'iptables -t nat -F' would flush all nat rules (might be too aggressive)
        // For now, let's just log or try to find a better way to clean up.
        info!(
            "Note: Manual cleanup of iptables MASQUERADE might be needed if multiple rules exist."
        );
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!("Get-NetIPInterface | Set-NetIPInterface -Forwarding Disabled");
        run_command("powershell", &["-Command", &ps_cmd])?;

        let ps_cleanup = "Remove-NetNat -Name 'VPN-Gateway-NAT' -Confirm:$false";
        let _ = run_command("powershell", &["-Command", ps_cleanup]);
    }

    info!("✅ Gateway mode disabled and restored to default.");
    Ok(())
}

fn get_default_interface() -> std::io::Result<String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("route")
            .args(&["-n", "get", "default"])
            .output()?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("interface:") {
                if let Some(iface) = trimmed.split_whitespace().nth(1) {
                    return Ok(iface.to_string());
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let output = Command::new("ip")
            .args(&["route", "show", "default"])
            .output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);
        // default via 192.168.1.1 dev eth0
        if let Some(dev_index) = output_str.split_whitespace().position(|s| s == "dev") {
            if let Some(iface) = output_str.split_whitespace().nth(dev_index + 1) {
                return Ok(iface.to_string());
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = "(Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric)[0].InterfaceAlias";
        let output = crate::command_ext::command_new("powershell")
            .args(&["-Command", ps_cmd])
            .output()?;
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    Ok("en0".to_string())
}

fn get_interface_ip(_iface: &str) -> std::io::Result<String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("ipconfig")
            .args(&["getifaddr", _iface])
            .output()?;
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    #[cfg(target_os = "linux")]
    {
        // simplified version
        let output = Command::new("hostname").arg("-I").output()?;
        return Ok(String::from_utf8_lossy(&output.stdout)
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!(
            "(Get-NetIPAddress -InterfaceAlias '{}' -AddressFamily IPv4).IPAddress",
            _iface
        );
        let output = crate::command_ext::command_new("powershell")
            .args(&["-Command", &ps_cmd])
            .output()?;
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    #[allow(unreachable_code)]
    Ok("unknown".to_string())
}

#[cfg(not(target_os = "windows"))]
fn set_sysctl(name: &str, value: &str) -> std::io::Result<()> {
    let status = Command::new("sysctl")
        .args(&["-w", &format!("{}={}", name, value)])
        .status()?;

    if !status.success() {
        error!("Failed to set sysctl {} to {}", name, value);
    }
    Ok(())
}

fn run_command(cmd: &str, args: &[&str]) -> std::io::Result<()> {
    debug!("Running command: {} {:?}", cmd, args);
    let output = {
        #[cfg(target_os = "windows")]
        {
            crate::command_ext::command_new(cmd).args(args).output()?
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new(cmd).args(args).output()?
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command {} failed: {}", cmd, stderr);
    }
    Ok(())
}
