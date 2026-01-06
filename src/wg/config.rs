use actix_web::{get, post, web, HttpResponse, Responder};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, default::Default, sync::Mutex};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Interface {
    private_key: String,
    listen_port: u16,
    address: String,
    dns: Vec<String>,
    mtu: u16,
}
#[derive(Serialize, Deserialize, Default, Debug)]
struct Peer {
    public_key: String,
    preshared_key: String,
    allowed_ips: Vec<String>,
    endpoint: String,
}
#[derive(Serialize, Deserialize, Default, Debug)]
struct Wg {
    interface: Interface,
    peers: Vec<Peer>,
}

#[derive(Debug, Default)]
pub struct WireGuardState {
    pub device_id: String,
    pub tun_fd: Option<i32>,
    pub handle: Option<i32>,
    pub payload: Option<WgConfigPayload>,
    pub config: Option<String>,
    pub status: ConnectionStatus,
    pub api_client: crate::api::remote_api::ApiClient,
    pub gateway_enabled: bool,
    pub original_gateway: Option<String>,
    pub active_endpoint: Option<String>,
}

impl WireGuardState {
    pub fn stop_and_cleanup(&mut self) -> Result<(), String> {
        info!("🔴 Performing global cleanup...");

        // 1. Turn off WireGuard tunnel
        if let Some(handle) = self.handle {
            info!("Turning off WireGuard tunnel (handle: {})", handle);
            crate::wg::WireGuardApi::turn_off(handle);
            self.handle = None;
        }

        // 2. Clear network configuration (routes, etc.)
        if let Err(e) = clear_network_config(self) {
            error!("Failed to clear network config: {}", e);
        }

        // 3. Close TUN FD
        if let Some(fd) = self.tun_fd {
            info!("Closing TUN device (FD: {})", fd);
            unsafe { libc::close(fd) };
            self.tun_fd = None;
        }

        // 4. Disable gateway mode if enabled
        if self.gateway_enabled {
            if let Err(e) = crate::interface::gateway::disable_gateway() {
                error!("Failed to disable gateway: {}", e);
            }
            self.gateway_enabled = false;
        }

        self.status = ConnectionStatus::Disconnected;
        self.active_endpoint = None;
        info!("✅ Cleanup completed.");
        Ok(())
    }

    pub fn update_endpoint(&mut self, new_endpoint_ip: &str) -> Result<(), String> {
        // 1. Update payload if it exists
        if let Some(payload) = &mut self.payload {
            let old_endpoint = &payload.peers.endpoint;
            let port = old_endpoint.split(':').nth(1).unwrap_or("51820");
            payload.peers.endpoint = format!("{}:{}", new_endpoint_ip, port);
            info!("Updated payload endpoint to: {}", payload.peers.endpoint);
        }

        // 2. Update config string if it exists
        if let Some(config) = &mut self.config {
            let lines: Vec<String> = config
                .lines()
                .map(|line| {
                    let trimmed = line.trim();
                    if trimmed.to_lowercase().starts_with("endpoint=") {
                        let old_val = trimmed.splitn(2, '=').nth(1).unwrap_or("");
                        let port = old_val.split(':').nth(1).unwrap_or("51820");
                        format!("endpoint={}:{}", new_endpoint_ip, port)
                    } else {
                        line.to_string()
                    }
                })
                .collect();
            *config = lines.join("\n");
            info!(
                "Updated config string endpoint to: {}:(preserved port or 51820)",
                new_endpoint_ip
            );
        }

        Ok(())
    }
    /// wg.conf → JSON
    /// 用来获取或修改配置
    pub fn wg_config_to_json(&mut self, conf: &str) -> Wg {
        let (mut wg, mut p, mut sec) = (Wg::default(), Peer::default(), "");
        for l in conf.lines().map(str::trim).filter(|l| !l.is_empty()) {
            if l.starts_with('[') {
                if sec == "Peer" {
                    wg.peers.push(p);
                    p = Peer::default()
                }
                sec = &l[1..l.len() - 1];
                continue;
            }
            let (k, v) = l.split_once('=').unwrap();
            let v = v.trim();
            match sec {
                "Interface" => match k.trim() {
                    "PrivateKey" => wg.interface.private_key = v.into(),
                    "ListenPort" => wg.interface.listen_port = v.parse().unwrap(),
                    "Address" => wg.interface.address = v.into(),
                    "DNS" => wg.interface.dns = v.split(',').map(|s| s.trim().into()).collect(),
                    "MTU" => wg.interface.mtu = v.parse().unwrap(),
                    _ => {}
                },
                "Peer" => match k.trim() {
                    "PublicKey" => p.public_key = v.into(),
                    "PresharedKey" => p.preshared_key = v.into(),
                    "AllowedIPs" => p.allowed_ips = v.split(',').map(|s| s.trim().into()).collect(),
                    "Endpoint" => p.endpoint = v.into(),
                    _ => {}
                },
                _ => {}
            }
        }
        if sec == "Peer" {
            wg.peers.push(p)
        }
        wg
    }

    // JSON -> wg.conf
    /// 可以用来直接给libwg-go使用
    pub fn json_to_wg_config(&self, w: &Wg) -> String {
        let mut s = format!(
            "private_key={}\nlisten_port={}\n",
            w.interface.private_key, w.interface.listen_port
        );

        // TODO: endpoint with port
        // TODO: multiple peers case
        // TODO: multiple allowed_ips case
        for p in &w.peers {
            s += &format!("public_key={}\n", p.public_key);
            if !p.preshared_key.is_empty() {
                s += &format!("preshared_key={}\n", p.preshared_key);
            }
            for ip in &p.allowed_ips {
                s += &format!("allowed_ips={}\n", ip);
            }
            if !p.endpoint.is_empty() {
                s += &format!("endpoint={}\n", p.endpoint);
            }
        }
        s
    }
}

#[derive(Deserialize, serde::Serialize, Debug, Clone, Default)]
#[allow(non_snake_case)] // Frontend sends camelCase
pub struct InterfaceSettings {
    pub address: String,
    pub listenPort: Option<u16>,
    pub privateKey: String,
    pub isTcp: bool,
    pub isServer: bool,
    pub isGlobal: bool,
}

#[derive(Deserialize, serde::Serialize, Debug, Clone, Default)]
#[allow(non_snake_case)]
pub struct PeerSettings {
    pub publicKey: String,
    pub presharedKey: Option<String>,
    pub allowedIPs: String,
    pub endpoint: String,
    pub isChangeRoute: bool,
}

#[derive(Deserialize, serde::Serialize, Debug, Clone, Default)]
pub struct WgConfigPayload {
    pub interface: InterfaceSettings,
    pub peers: PeerSettings,
}

#[derive(serde::Serialize)]
pub struct WgStatsResponse {
    pub rx: u64,
    pub tx: u64,
    pub peers: u32,
    pub status: ConnectionStatus,
    pub gateway_enabled: bool,
}

#[get("/api/getwgstats")]
pub async fn get_wg_stats(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let (handle, status) = match state.lock() {
        Ok(s) => match s.handle {
            Some(h) => (h, s.status),
            None => {
                return HttpResponse::Ok().json(WgStatsResponse {
                    rx: 0,
                    tx: 0,
                    peers: 0,
                    status: s.status,
                    gateway_enabled: s.gateway_enabled,
                })
            }
        },
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    match crate::wg::WireGuardApi::get_stats(handle) {
        Ok((rx, tx)) => {
            let gateway_enabled = match state.lock() {
                Ok(s) => s.gateway_enabled,
                Err(_) => false,
            };
            HttpResponse::Ok().json(WgStatsResponse {
                rx,
                tx,
                peers: 1,
                status,
                gateway_enabled,
            })
        }
        Err(e) => {
            error!("Failed to get stats: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to get stats: {}", e))
        }
    }
}

#[post("/api/setwg")]
pub async fn set_wg_config(
    config: web::Json<WgConfigPayload>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    debug!("Received WG Config: {:?}", config);

    let mut state = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    match apply_wg_config(&mut state) {
        Ok(_) => {
            // Save payload to state
            state.payload = Some(config.into_inner().clone());

            // Serialize payload to JSON for persistence
            if let Some(payload) = &state.payload {
                match serde_json::to_string(payload) {
                    Ok(json_str) => {
                        if let Err(e) = crate::wg::store::save_config(&json_str) {
                            error!("Failed to persist config: {}", e);
                        } else {
                            info!("Config persisted successfully.");
                        }
                    }
                    Err(e) => error!("Failed to serialize payload: {}", e),
                }
            }

            HttpResponse::Ok().body("WireGuard configured successfully")
        }
        Err(e) => {
            error!("Failed to apply WireGuard config: {}", e);
            HttpResponse::InternalServerError()
                .body(format!("Failed to apply WireGuard config: {}", e))
        }
    }
}

#[post("/api/gateway/on")]
pub async fn enable_gateway_api(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let mut state = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    if state.status != ConnectionStatus::Connected {
        return HttpResponse::BadRequest().body("VPN must be connected to enable gateway mode");
    }

    match crate::interface::gateway::enable_gateway("utun9981") {
        Ok(_) => {
            state.gateway_enabled = true;
            HttpResponse::Ok().body("Gateway mode enabled")
        }
        Err(e) => {
            error!("Failed to enable gateway: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to enable gateway: {}", e))
        }
    }
}

#[post("/api/gateway/off")]
pub async fn disable_gateway_api(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let mut state = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    match crate::interface::gateway::disable_gateway() {
        Ok(_) => {
            state.gateway_enabled = false;
            HttpResponse::Ok().body("Gateway mode disabled")
        }
        Err(e) => {
            error!("Failed to disable gateway: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to disable gateway: {}", e))
        }
    }
}

#[get("/api/gateway/status")]
pub async fn gateway_status_api(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let state = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    HttpResponse::Ok().json(serde_json::json!({
        "enabled": state.gateway_enabled
    }))
}

fn remove_keys(input: &str, keys: &[&str]) -> String {
    let remove: HashSet<&str> = keys.iter().copied().collect();

    input
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            match line.split_once('=') {
                Some((key, _)) => !remove.contains(key),
                None => true, // 非 key=value 行，保留
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn apply_wg_config(state: &mut WireGuardState) -> Result<i32, String> {
    // 0. Initial preparation of config string
    let mut config = state.config.clone().unwrap_or_default();
    state.status = ConnectionStatus::Connecting;
    // 1. Ensure TUN device exists
    if state.tun_fd.is_none() {
        match crate::wg::WireGuardApi::create_tun("utun9981", 1420) {
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

    // 2. Handle wireguard config to turn on wg
    info!("WireGuard config: {}", config);

    let ip = "";

    if let Some(fd) = state.tun_fd {
        info!("Turning on WireGuard with FD: {}", fd);
        match crate::wg::WireGuardApi::turn_on(&config, fd) {
            Ok(handle) => {
                info!("WireGuard turned on successfully. Handle: {}", handle);
                state.handle = Some(handle);

                // 3. Configure Network (IP and Routes)
                // Extract endpoint IP (handling IPv4 and IPv6 with optional port)
                let endpoint = config
                    .lines()
                    .map(str::trim)
                    .find(|line| line.to_lowercase().starts_with("endpoint="))
                    .and_then(|line| line.splitn(2, '=').nth(1))
                    .map(|v| v.trim().to_string())
                    .unwrap();
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
                        &endpoint
                    }
                } else {
                    &endpoint
                };

                // Detect gateway for the endpoint IP
                let gateway_ip = match get_gateway_for_ip(endpoint_ip) {
                    Ok(ip) => ip,
                    Err(e) => {
                        error!("Failed to detect gateway for {}: {}", endpoint_ip, e);
                        // Fallback to default gateway
                        get_gateway_for_ip("").unwrap_or_else(|_| "192.168.1.1".to_string())
                    }
                };

                // Backup original gateway if not already set
                if state.original_gateway.is_none() {
                    state.original_gateway = Some(gateway_ip.clone());
                    info!("Backed up original gateway: {}", gateway_ip);
                }

                info!(
                    "Using gateway: {} for endpoint: {}",
                    gateway_ip, endpoint_ip
                );

                if let Err(e) = configure_network("utun9981", &ip, &gateway_ip, endpoint_ip) {
                    error!("Failed to configure network: {}, config param addr:{}, gateway :{}, endpoint: {}", e, &ip, &gateway_ip, endpoint_ip);
                    state.status = ConnectionStatus::Error;
                } else {
                    state.status = ConnectionStatus::Connected;
                    state.active_endpoint = Some(endpoint_ip.to_string());

                    // 4. Persist config for future use
                    if let Some(payload) = &state.payload {
                        match serde_json::to_string(payload) {
                            Ok(json_str) => {
                                if let Err(e) = crate::wg::store::save_config(&json_str) {
                                    error!("Failed to persist config: {}", e);
                                } else {
                                    info!("✅ Config persisted successfully.");
                                }
                            }
                            Err(e) => error!("Failed to serialize payload: {}", e),
                        }
                    } else {
                        error!("Don't save wg config to use in future");
                    }
                }
                // TODO: this is only tmp info
                error!("this config will be saved: {}", config);
                Ok(handle)
            }
            Err(e) => {
                state.status = ConnectionStatus::Error;
                Err(format!("Failed to turn on WireGuard: {}", e))
            }
        }
    } else {
        state.status = ConnectionStatus::Error;
        Err("TUN device initialization failed".to_string())
    }
}

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

        // Priority 1: Use gateway IP if it's not a tunnel
        if let Some(gw) = gateway {
            if !is_tunnel(&gw) {
                return Ok(gw);
            }
        }

        // Priority 2: Use interface name if it's not a tunnel
        if let Some(iface) = interface {
            if !is_tunnel(&iface) {
                return Ok(iface);
            }
        }

        // Fallback: If current route is through a tunnel, try to find the system's default physical gateway
        if !target.is_empty() {
            return get_gateway_for_ip("");
        }

        // Final attempt for macOS: parse netstat to find a non-tunnel default route
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
        // Use 'ip route get' to find the route for a specific target more accurately
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

        // Fallback to searching default route
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
    ip: &str,
    gateway: &str,
    endpoint_ip: &str,
) -> std::io::Result<()> {
    info!("Configuring network for interface: {}", interface_name);

    let skip_base = ip == "SKIP";

    #[cfg(target_os = "macos")]
    {
        if !skip_base {
            // Parse IPs (could be comma separated)
            for addr in ip.split(',') {
                let addr = addr.trim();
                if addr.is_empty() {
                    continue;
                }
                let is_ipv6 = addr.contains(':');
                let ip_only = addr.split('/').next().unwrap_or(addr);

                if is_ipv6 {
                    // ifconfig utun9981 inet6 fd00::1 fd00::1 prefixlen 128
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
                    // ifconfig utun9981 10.99.0.7 10.99.0.7 netmask 255.255.0.0
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

            let routes = vec![
                ("0.0.0.0/1", interface_name, "-inet", true),
                ("128.0.0.0/1", interface_name, "-inet", true),
                // ("10.99.0.0/24", "10.99.0.7", "-inet", false), // via IP
                // ("10.99.0.7/32", "10.99.0.7", "-inet", false), // via IP
                ("224.0.0.0/4", interface_name, "-inet", true), // -interface
                ("255.255.255.255", interface_name, "-inet", true), // -interface (no CIDR)
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

                info!("Executing route command: {:?}", cmd);
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

        // Add specific route for the endpoint to go through the physical gateway
        if !endpoint_ip.is_empty() {
            let is_v6 = endpoint_ip.contains(':');
            let family = if is_v6 { "-inet6" } else { "-inet" };

            let mut cmd = std::process::Command::new("route");
            cmd.arg("-n")
                .arg("add")
                .arg(family)
                .arg("-host")
                .arg(endpoint_ip);

            // If gateway is an interface name (doesn't look like an IP), we need the -interface flag
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
            // ip link set dev <iface> up
            std::process::Command::new("ip")
                .args(&["link", "set", "dev", interface_name, "up"])
                .output()?;

            // ip addr add <ip>/32 dev <iface>
            // Assuming /16 based on netmask 255.255.0.0 from macOS example
            std::process::Command::new("ip")
                .args(&["addr", "add", &format!("{}/16", ip), "dev", interface_name])
                .output()?;

            let routes = vec!["10.99.0.0/16", "0.0.0.0/1", "128.0.0.0/1"];

            for dest in routes {
                std::process::Command::new("ip")
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
            cmd.output()?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if !skip_base {
            // netsh interface ip set address name="<iface>" source=static addr=<ip> mask=255.255.0.0
            std::process::Command::new("netsh")
                .args(&[
                    "interface",
                    "ip",
                    "set",
                    "address",
                    &format!("name=\"{}\"", interface_name),
                    "source=static",
                    &format!("addr={}", ip),
                    "mask=255.255.0.0",
                ])
                .output()?;

            // route add 0.0.0.0 mask 128.0.0.0 <ip> metric 1
            // Windows route add syntax: route ADD [destination] MASK [mask] [gateway] [metric] IF [interface]
            // We need to be careful with syntax.

            // 10.99.0.0/16
            std::process::Command::new("route")
                .args(&[
                    "ADD",
                    "10.99.0.0",
                    "MASK",
                    "255.255.0.0",
                    "0.0.0.0",
                    "IF",
                    interface_name,
                ]) // 0.0.0.0 as gateway for on-link?
                .output()?;

            // 0.0.0.0/1 -> 0.0.0.0 mask 128.0.0.0
            std::process::Command::new("route")
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

            // 128.0.0.0/1 -> 128.0.0.0 mask 128.0.0.0
            std::process::Command::new("route")
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
            std::process::Command::new("route")
                .args(&["ADD", endpoint_ip, "MASK", "255.255.255.255", gateway])
                .output()?;
        }
    }

    Ok(())
}

pub fn clear_network_config(state: &mut WireGuardState) -> std::io::Result<()> {
    let interface_name = "utun9981";
    info!(
        "Clearing network configuration for interface: {}",
        interface_name
    );

    let mut endpoint_ip = "";
    if let Some(payload) = &state.payload {
        let endpoint = payload.peers.endpoint.as_str();
        endpoint_ip = if let Some(last_colon) = endpoint.rfind(':') {
            let port_str = &endpoint[last_colon + 1..];
            if port_str.parse::<u16>().is_ok() {
                let host = &endpoint[..last_colon];
                if host.starts_with('[') && host.ends_with(']') {
                    &host[1..host.len() - 1]
                } else {
                    host
                }
            } else {
                endpoint
            }
        } else {
            endpoint
        };
    }

    #[cfg(target_os = "macos")]
    {
        let routes = vec![
            ("0.0.0.0/1", "-inet"),
            ("128.0.0.0/1", "-inet"),
            ("default", "-inet"),
            ("0.0.0.0/0", "-inet"),
            ("10.99.0.0/24", "-inet"),
            //("10.99.0.7/32", "-inet"), // TODO don't use hardcode IP
            ("224.0.0.0/4", "-inet"),
            ("255.255.255.255", "-inet"),
            ("::/1", "-inet6"),
            ("8000::/1", "-inet6"),
            ("10.99.0.0/16", "-inet"), // Old route cleanup
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

        // Restore original default route if available
        if let Some(gw) = &state.original_gateway {
            info!("Restoring original default route via {}", gw);
            let mut cmd = std::process::Command::new("route");
            cmd.arg("-n").arg("add").arg("-inet").arg("default");

            // If gateway is an interface name (doesn't look like an IP), we need the -interface flag
            if !gw.contains('.') && !gw.contains(':') {
                cmd.arg("-interface");
            }
            cmd.arg(gw);

            let output = cmd.output()?;
            if !output.status.success() {
                error!(
                    "Failed to restore default route: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            } else {
                info!("Successfully restored default route.");
            }

            state.original_gateway = None;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wg_config_to_json() {
        let mut state = WireGuardState::default();
        let config = "[Interface]\nPrivateKey = private_key_123\nListenPort = 51820\nAddress = 10.0.0.1/24\nDNS = 1.1.1.1, 8.8.8.8\nMTU = 1420\n\n[Peer]\nPublicKey = public_key_abc\nPresharedKey = preshared_key_def\nAllowedIPs = 0.0.0.0/0, ::/0\nEndpoint = 1.2.3.4:51820\n";
        let wg = state.wg_config_to_json(config);

        println!("wg jso is : {:#?}", wg);
        assert_eq!(wg.interface.private_key, "private_key_123");
        assert_eq!(wg.interface.listen_port, 51820);
        assert_eq!(wg.interface.address, "10.0.0.1/24");
        assert_eq!(wg.interface.dns, vec!["1.1.1.1", "8.8.8.8"]);
        assert_eq!(wg.interface.mtu, 1420);

        assert_eq!(wg.peers.len(), 1);
        assert_eq!(wg.peers[0].public_key, "public_key_abc");
        assert_eq!(wg.peers[0].preshared_key, "preshared_key_def");
        assert_eq!(wg.peers[0].allowed_ips, vec!["0.0.0.0/0", "::/0"]);
        assert_eq!(wg.peers[0].endpoint, "1.2.3.4:51820");
    }

    #[test]
    fn test_json_to_wg_config() {
        let state = WireGuardState::default();
        let wg = Wg {
            interface: Interface {
                private_key: "private_key_123".into(),
                listen_port: 51820,
                address: "10.0.0.1/24".into(),
                dns: vec!["1.1.1.1".into(), "8.8.8.8".into()],
                mtu: 1420,
            },
            peers: vec![Peer {
                public_key: "public_key_abc".into(),
                preshared_key: "preshared_key_def".into(),
                allowed_ips: vec!["0.0.0.0/0".into(), "::/0".into()],
                endpoint: "1.2.3.4:51820".into(),
            }],
        };

        let config = state.json_to_wg_config(&wg);

        println!("test_json_to_wg_config is : {:#?}", config);
        assert!(config.contains("private_key = private_key_123"));
        assert!(config.contains("listen_port = 51820"));
        assert!(config.contains("public_key=public_key_abc"));
        assert!(config.contains("preshared_key=preshared_key_def"));
        assert!(config.contains("allowed_ips=0.0.0.0/0"));
        assert!(config.contains("allowed_ips=::/0"));
        assert!(config.contains("endpoint=1.2.3.4:51820"));
    }
}
