//! WireGuard **client**-mode networking helpers.
//!
//! These functions used to live in `wg::config`, mixed in with serialization
//! and shared state. They were moved here so that the server-mode path
//! (`wg::server`) can stay free of routing / gateway-detection / leak-blocking
//! machinery that only makes sense for outbound clients.
//!
//! Public surface:
//!
//! - [`apply_wg_config`]: full client connect routine (TUN, libwg-go, default
//!   route hijack, host route to the endpoint via the physical gateway).
//! - [`clear_network_config`]: tears down everything that `apply_wg_config`
//!   installed. Called by `WireGuardState::stop_and_cleanup`.
//!
//! The example INI snippet that motivates this module:
//!
//! ```text
//! [Interface]
//! Address = 10.88.0.2/24
//! ListenPort = 51820
//! PrivateKey = ...
//!
//! [Peer]
//! PublicKey = ...
//! PresharedKey = ...
//! AllowedIPs = 0.0.0.0/0, ::/0
//! Endpoint = 1.2.3.4:51820
//! ```

use crate::wg::config::{ConnectionStatus, WireGuardState};
use crate::wg::WireGuardApi;
use log::{debug, error, info, warn};

/// Hard-coded TUN name used by the client. Kept identical to the legacy code
/// so that existing routes survive an upgrade.
pub const CLIENT_IFACE: &str = "utun9981";

pub fn apply_wg_config(state: &mut WireGuardState) -> Result<i32, String> {
    state.status = ConnectionStatus::Connecting;

    let _wg = if let Some(wg) = state.wg_config.clone() {
        info!("Using pre-populated WireGuard config");
        wg
    } else {
        let config = state.config.clone().unwrap_or_default();
        state.wg_config_to_json(&config)
    };

    if state.tun_fd.is_none() {
        WireGuardApi::set_logger();
        match WireGuardApi::create_tun(CLIENT_IFACE, 1280) {
            Ok(fd) => {
                info!("TUN device created successfully. FD: {}", fd);
                state.tun_fd = Some(fd);
            }
            Err(e) => {
                state.status = ConnectionStatus::Error;
                return Err(format!("Failed to create TUN device: {}", e));
            }
        }
    }

    let tun_ip = &_wg.interface.address;
    let config = state.json_to_wg_config(&_wg);

    let fd = match state.tun_fd {
        Some(f) => f,
        None => {
            state.status = ConnectionStatus::Error;
            return Err("TUN device initialization failed".to_string());
        }
    };

    let handle = match WireGuardApi::turn_on(&config, fd) {
        Ok(h) => h,
        Err(e) => {
            state.status = ConnectionStatus::Error;
            return Err(format!("Failed to turn on WireGuard: {}", e));
        }
    };
    info!("WireGuard turned on successfully. Handle: {}", handle);
    state.handle = Some(handle);

    // AmneziaWG uapi lines are emitted only when `state.enhance_mode.obfuscate` is true
    // (same flag as the Overview / enhance-mode obfuscation switch); `json_to_wg_config` reads that.
    WireGuardApi::enable_interference_detection(handle, state.enhance_mode.obfuscate);

    let endpoint = config
        .lines()
        .map(str::trim)
        .find(|line| line.to_lowercase().starts_with("endpoint="))
        .and_then(|line| line.splitn(2, '=').nth(1))
        .map(|v| v.trim().to_string())
        .unwrap_or_default();

    let endpoint_ip = if let Some(last_colon) = endpoint.rfind(':') {
        let port_str = &endpoint[last_colon + 1..];
        if port_str.parse::<u16>().is_ok() {
            let host = &endpoint[..last_colon];
            if host.starts_with('[') && host.ends_with(']') {
                &host[1..host.len() - 1]
            } else {
                host
            }
        } else {
            endpoint.as_str()
        }
    } else {
        endpoint.as_str()
    };

    if state.original_gateway_v4.is_none() {
        let targets = ["8.8.8.8", "1.1.1.1", ""];
        for target in targets {
            match get_gateway_for_ip(target) {
                Ok(gw4) => {
                    state.original_gateway_v4 = Some(gw4.clone());
                    info!("Backed up original IPv4 gateway: {}", gw4);
                    break;
                }
                Err(e) => debug!("Failed to backup IPv4 gateway via {}: {}", target, e),
            }
        }
        if state.original_gateway_v4.is_none() {
            warn!("Could not backup any original IPv4 default gateway!");
        }
    }

    if state.original_gateway_v6.is_none() {
        let targets = ["2001:4860:4860::8888", "2606:4700:4700::1111", ""];
        for target in targets {
            match get_gateway_for_ip(target) {
                Ok(gw6) => {
                    state.original_gateway_v6 = Some(gw6.clone());
                    info!("Backed up original IPv6 gateway: {}", gw6);
                    break;
                }
                Err(e) => debug!("Failed to backup IPv6 gateway via {}: {}", target, e),
            }
        }
        if state.original_gateway_v6.is_none() {
            warn!("Could not backup any original IPv6 default gateway!");
        }
    }

    let gateway_ip = match get_gateway_for_ip(endpoint_ip) {
        Ok(ip) => ip,
        Err(e) => {
            error!("Failed to detect gateway for {}: {}", endpoint_ip, e);
            state
                .original_gateway_v4
                .clone()
                .unwrap_or_else(|| "192.168.1.1".to_string())
        }
    };
    info!(
        "Using gateway: {} for endpoint: {}",
        gateway_ip, endpoint_ip
    );

    if let Err(e) = configure_network(CLIENT_IFACE, tun_ip, &gateway_ip, endpoint_ip) {
        error!(
            "Failed to configure network: {}, addr:{}, gateway:{}, endpoint:{}",
            e, tun_ip, gateway_ip, endpoint_ip
        );
        state.status = ConnectionStatus::Error;
    } else {
        state.status = ConnectionStatus::Connected;
        state.active_endpoint = Some(endpoint_ip.to_string());
        state.wg_config = Some(_wg);
    }
    Ok(handle)
}

pub fn clear_network_config(state: &mut WireGuardState) -> std::io::Result<()> {
    let interface_name = CLIENT_IFACE;
    info!(
        "Clearing network configuration for interface: {}",
        interface_name
    );

    let endpoint_ip_str = state.active_endpoint.clone().unwrap_or_default();
    let endpoint_ip = endpoint_ip_str.as_str();

    #[cfg(target_os = "macos")]
    {
        let routes = vec![
            ("0.0.0.0/1", "-inet"),
            ("128.0.0.0/1", "-inet"),
            ("10.99.0.0/24", "-inet"),
            ("224.0.0.0/4", "-inet"),
            ("255.255.255.255", "-inet"),
            ("::/1", "-inet6"),
            ("8000::/1", "-inet6"),
            ("10.99.0.0/16", "-inet"),
        ];
        for (dest, family) in routes {
            let mut cmd = std::process::Command::new("route");
            cmd.arg("-n").arg("delete").arg(family);
            if dest.contains('/') || dest == "default" {
                cmd.arg("-net").arg(dest);
            } else {
                cmd.arg("-host").arg(dest);
            }
            let _ = cmd.output();
        }
        state.original_gateway_v4 = None;
        state.original_gateway_v6 = None;

        if !endpoint_ip.is_empty() {
            let is_v6 = endpoint_ip.contains(':');
            let family = if is_v6 { "-inet6" } else { "-inet" };
            let _ = std::process::Command::new("route")
                .arg("-n")
                .arg("delete")
                .arg(family)
                .arg("-host")
                .arg(endpoint_ip)
                .output();
        }
    }

    #[cfg(target_os = "linux")]
    {
        let routes = vec!["0.0.0.0/1", "128.0.0.0/1", "10.99.0.0/16"];
        for dest in routes {
            let _ = std::process::Command::new("ip")
                .args(&["route", "del", dest])
                .output();
        }
        if !endpoint_ip.is_empty() {
            let _ = std::process::Command::new("ip")
                .args(&["route", "del", endpoint_ip])
                .output();
        }
    }

    #[cfg(target_os = "windows")]
    {
        let routes = vec![
            ("0.0.0.0", "128.0.0.0"),
            ("128.0.0.0", "128.0.0.0"),
            ("10.99.0.0", "255.255.0.0"),
        ];
        for (dest, mask) in routes {
            let _ = std::process::Command::new("route")
                .args(&["DELETE", dest, "MASK", mask])
                .output();
        }
        if !endpoint_ip.is_empty() {
            let _ = std::process::Command::new("route")
                .args(&["DELETE", endpoint_ip])
                .output();
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers (private to client mode)
// ---------------------------------------------------------------------------

fn get_gateway_for_ip(target: &str) -> std::io::Result<String> {
    #[cfg(target_os = "macos")]
    {
        let is_ipv6 = target.contains(':');
        let mut args = vec!["-n", "get"];
        if is_ipv6 {
            args.push("-inet6");
        } else {
            args.push("-inet");
        }
        let target_arg = if target.is_empty() { "default" } else { target };
        args.push(target_arg);

        let output = std::process::Command::new("route").args(&args).output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);

        let mut gateway = None;
        let mut interface = None;
        for line in output_str.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("gateway:") {
                gateway = trimmed.split_whitespace().nth(1).map(|s| s.to_string());
            } else if trimmed.starts_with("interface:") {
                interface = trimmed.split_whitespace().nth(1).map(|s| s.to_string());
            }
        }

        let is_tunnel = |name: &str| name.starts_with("utun") || name.starts_with("tun");

        if let Some(gw) = gateway {
            if !is_tunnel(&gw) {
                return Ok(gw);
            }
        }
        if let Some(iface) = interface {
            if !is_tunnel(&iface) {
                return Ok(iface);
            }
        }
        if !target.is_empty() {
            return get_gateway_for_ip("");
        }

        let netstat_output = std::process::Command::new("netstat")
            .args(&["-rn", "-f", if is_ipv6 { "inet6" } else { "inet" }])
            .output()?;
        let netstat_str = String::from_utf8_lossy(&netstat_output.stdout);
        for line in netstat_str.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2
                && (parts[0] == "default" || parts[0] == "0.0.0.0/0" || parts[0] == "::/0")
            {
                let gw = parts[1];
                let iface = if parts.len() >= 4 { parts[3] } else { "" };
                if !is_tunnel(gw) && !is_tunnel(iface) {
                    return Ok(gw.to_string());
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No physical gateway found",
        ))
    }

    #[cfg(target_os = "linux")]
    {
        let target_arg = if target.is_empty() { "8.8.8.8" } else { target };
        let output = std::process::Command::new("ip")
            .args(&["route", "get", target_arg])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let output_str = String::from_utf8_lossy(&out.stdout);
                let is_tunnel = |s: &str| s.contains("tun") || s.contains("utun");
                let parts: Vec<&str> = output_str.split_whitespace().collect();

                if let Some(via_index) = parts.iter().position(|&x| x == "via") {
                    if let Some(gateway) = parts.get(via_index + 1) {
                        if !is_tunnel(gateway) {
                            return Ok(gateway.to_string());
                        }
                    }
                }
                if let Some(dev_index) = parts.iter().position(|&x| x == "dev") {
                    if let Some(iface) = parts.get(dev_index + 1) {
                        if !is_tunnel(iface) {
                            return Ok(iface.to_string());
                        }
                    }
                }
            }
        }

        let output = std::process::Command::new("ip")
            .args(&["route", "show", "default"])
            .output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("tun") || line.contains("utun") {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(via_index) = parts.iter().position(|&x| x == "via") {
                if let Some(gateway) = parts.get(via_index + 1) {
                    return Ok(gateway.to_string());
                }
            }
            if let Some(dev_index) = parts.iter().position(|&x| x == "dev") {
                if let Some(iface) = parts.get(dev_index + 1) {
                    return Ok(iface.to_string());
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Gateway not found",
        ))
    }

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("powershell")
            .args(&["-Command", "Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Select-Object -ExpandProperty NextHop"])
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get default route",
            ));
        }
        let output_str = String::from_utf8_lossy(&output.stdout);
        let gateway = output_str.trim();
        if !gateway.is_empty() {
            return Ok(gateway.to_string());
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Gateway not found in PowerShell output",
        ))
    }
}

fn configure_network(
    interface_name: &str,
    tun_ip: &str,
    gateway: &str,
    endpoint_ip: &str,
) -> std::io::Result<()> {
    info!("Configuring network for interface: {}", interface_name);
    let skip_base = tun_ip == "SKIP";

    #[cfg(target_os = "macos")]
    {
        if !skip_base {
            for addr in tun_ip.split(',') {
                let addr = addr.trim();
                if addr.is_empty() {
                    continue;
                }
                let is_ipv6 = addr.contains(':');
                let ip_only = addr.split('/').next().unwrap_or(addr);

                if is_ipv6 {
                    let output = std::process::Command::new("ifconfig")
                        .arg(interface_name)
                        .arg("inet6")
                        .arg(ip_only)
                        .arg(ip_only)
                        .arg("prefixlen")
                        .arg("128")
                        .output()?;
                    if !output.status.success() {
                        error!(
                            "ifconfig inet6 failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                } else {
                    let output = std::process::Command::new("ifconfig")
                        .arg(interface_name)
                        .arg(ip_only)
                        .arg(ip_only)
                        .arg("netmask")
                        .arg("255.255.0.0")
                        .output()?;
                    if !output.status.success() {
                        error!(
                            "ifconfig ipv4 failed: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                }
            }

            // Default-route hijack, plus broadcast/multicast leak-block to match
            // wg-quick behavior on macOS.
            let routes = vec![
                ("0.0.0.0/1", interface_name, "-inet", true),
                ("128.0.0.0/1", interface_name, "-inet", true),
                ("224.0.0.0/4", interface_name, "-inet", true),
                ("255.255.255.255", interface_name, "-inet", true),
                ("::/1", interface_name, "-inet6", true),
                ("8000::/1", interface_name, "-inet6", true),
            ];
            for (dest, target, family, is_interface) in routes {
                let mut cmd = std::process::Command::new("route");
                cmd.arg("-n").arg("add").arg(family);
                if dest.contains('/') || dest == "default" {
                    cmd.arg("-net").arg(dest);
                } else {
                    cmd.arg("-host").arg(dest);
                }
                if is_interface {
                    cmd.arg("-interface");
                }
                cmd.arg(target);
                debug!("Executing route command: {:?}", cmd);
                let output = cmd.output()?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("File exists") {
                        debug!("Route {} already exists, skipping.", dest);
                    } else {
                        error!("Failed to add route {} ({}): {}", dest, family, stderr);
                    }
                } else {
                    info!("Successfully added route: {} via {}", dest, target);
                }
            }
        }

        if !endpoint_ip.is_empty() {
            let is_v6 = endpoint_ip.contains(':');
            let family = if is_v6 { "-inet6" } else { "-inet" };
            let mut cmd = std::process::Command::new("route");
            cmd.arg("-n")
                .arg("add")
                .arg(family)
                .arg("-host")
                .arg(endpoint_ip);
            if !gateway.contains('.') && !gateway.contains(':') {
                cmd.arg("-interface");
            }
            cmd.arg(gateway);
            let output = cmd.output()?;
            if !output.status.success() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "route add host failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if !skip_base {
            let _ = std::process::Command::new("ip")
                .args(&["link", "set", "dev", interface_name, "up"])
                .output()?;

            // Use the first IP from a possibly comma-separated list.
            for addr in tun_ip.split(',') {
                let addr = addr.trim();
                if addr.is_empty() {
                    continue;
                }
                let cidr = if addr.contains('/') {
                    addr.to_string()
                } else if addr.contains(':') {
                    format!("{}/128", addr)
                } else {
                    format!("{}/32", addr)
                };
                let _ = std::process::Command::new("ip")
                    .args(&["addr", "add", &cidr, "dev", interface_name])
                    .output()?;
            }

            let routes = vec!["10.99.0.0/16", "0.0.0.0/1", "128.0.0.0/1"];
            for dest in routes {
                let _ = std::process::Command::new("ip")
                    .args(&["route", "add", dest, "dev", interface_name])
                    .output()?;
            }
        }

        if !endpoint_ip.is_empty() {
            let mut cmd = std::process::Command::new("ip");
            cmd.arg("route").arg("add").arg(endpoint_ip);
            if gateway.contains('.') || gateway.contains(':') {
                cmd.arg("via").arg(gateway);
            } else {
                cmd.arg("dev").arg(gateway);
            }
            let _ = cmd.output()?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if !skip_base {
            let first_addr = tun_ip
                .split(',')
                .next()
                .unwrap_or("")
                .split('/')
                .next()
                .unwrap_or("");
            let _ = std::process::Command::new("netsh")
                .args(&[
                    "interface",
                    "ip",
                    "set",
                    "address",
                    &format!("name=\"{}\"", interface_name),
                    "source=static",
                    &format!("addr={}", first_addr),
                    "mask=255.255.0.0",
                ])
                .output()?;

            let _ = std::process::Command::new("route")
                .args(&[
                    "ADD",
                    "10.99.0.0",
                    "MASK",
                    "255.255.0.0",
                    "0.0.0.0",
                    "IF",
                    interface_name,
                ])
                .output()?;
            let _ = std::process::Command::new("route")
                .args(&[
                    "ADD",
                    "0.0.0.0",
                    "MASK",
                    "128.0.0.0",
                    "0.0.0.0",
                    "IF",
                    interface_name,
                ])
                .output()?;
            let _ = std::process::Command::new("route")
                .args(&[
                    "ADD",
                    "128.0.0.0",
                    "MASK",
                    "128.0.0.0",
                    "0.0.0.0",
                    "IF",
                    interface_name,
                ])
                .output()?;
        }

        if !endpoint_ip.is_empty() {
            let _ = std::process::Command::new("route")
                .args(&["ADD", endpoint_ip, "MASK", "255.255.255.255", gateway])
                .output()?;
        }
    }

    Ok(())
}
