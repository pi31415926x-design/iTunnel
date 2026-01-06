use log::debug;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::net::TcpSocket;
use tokio::process::Command;
use tokio::time::timeout;

#[cfg(target_os = "macos")]
use std::os::unix::io::AsRawFd;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub id: String,
    pub location: String,
    pub ip4: String,
    pub ip6: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SpeedTestResult {
    pub id: String,
    pub location: String,
    pub ip4: String,
    pub ip6: String,
    pub latency4_ms: Option<u128>,
    pub latency6_ms: Option<u128>,
}

pub fn get_servers() -> Vec<ServerInfo> {
    vec![
        ServerInfo {
            id: "001".to_string(),
            location: "Korea".to_string(),
            ip4: "52.78.213.238".to_string(),
            ip6: "2406:da12:c88:9100:2193:38b4:6050:bcec".to_string(),
        },
        ServerInfo {
            id: "002".to_string(),
            location: "Japan".to_string(),
            ip4: "54.249.221.90".to_string(),
            ip6: "2406:da14:2ea:4600:ade3:d9e5:e626:a1af".to_string(),
        },
        ServerInfo {
            id: "003".to_string(),
            location: "Japan".to_string(),
            ip4: "13.231.209.151".to_string(),
            ip6: "2406:da14:2ea:4600:88b9:3372:8c5b:68ce".to_string(),
        },
        ServerInfo {
            id: "004".to_string(),
            location: "US West".to_string(),
            ip4: "35.91.75.187".to_string(),
            ip6: "2600:1f14:1605:a100:e999:15fb:7f66:1772".to_string(),
        },
    ]
}

#[cfg(target_os = "macos")]
struct PhysicalInterfaceInfo {
    index: u32,
    name: String,
    gateway_v4: Option<String>,
    gateway_v6: Option<String>,
}

#[cfg(target_os = "macos")]
fn get_physical_interface_info() -> Option<PhysicalInterfaceInfo> {
    let interfaces = NetworkInterface::show().ok()?;

    let output = std::process::Command::new("netstat")
        .args(&["-nr"])
        .output()
        .ok()?;

    let s = String::from_utf8_lossy(&output.stdout);
    let mut iface_name = None;
    let mut gateway_v4 = None;
    let mut gateway_v6 = None;

    let mut current_family = "";
    for line in s.lines() {
        if line.contains("Internet:") {
            current_family = "v4";
            continue;
        } else if line.contains("Internet6:") {
            current_family = "v6";
            continue;
        }

        if line.contains("default") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let gateway = parts[1].to_string();
                let name = parts[parts.len() - 1];
                // Skip virtual/VPN interfaces, focus on enX or ethX
                if !name.starts_with("utun")
                    && !name.starts_with("wg")
                    && (name.starts_with("en") || name.starts_with("eth"))
                {
                    iface_name = Some(name.to_string());
                    if current_family == "v4" {
                        gateway_v4 = Some(gateway);
                    } else if current_family == "v6" {
                        gateway_v6 = Some(gateway);
                    }
                }
            }
        }
    }

    if let Some(name) = iface_name {
        if let Some(itf) = interfaces.iter().find(|i| i.name == name) {
            return Some(PhysicalInterfaceInfo {
                index: itf.index,
                name: itf.name.clone(),
                gateway_v4,
                gateway_v6,
            });
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn manage_bypass_routes(servers: &[ServerInfo], info: &PhysicalInterfaceInfo, add: bool) {
    let action = if add { "add" } else { "delete" };
    for s in servers {
        if let Some(gw) = &info.gateway_v4 {
            if !s.ip4.is_empty() {
                let _ = std::process::Command::new("route")
                    .arg("-n")
                    .arg(action)
                    .arg("-inet")
                    .arg("-host")
                    .arg(&s.ip4)
                    .arg(gw)
                    .output();
            }
        }
        if let Some(gw) = &info.gateway_v6 {
            if !s.ip6.is_empty() {
                let _ = std::process::Command::new("route")
                    .arg("-n")
                    .arg(action)
                    .arg("-inet6")
                    .arg("-host")
                    .arg(&s.ip6)
                    .arg(gw)
                    .output();
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn bind_socket_to_interface(
    fd: std::os::unix::io::RawFd,
    interface_index: u32,
    is_v6: bool,
) -> std::io::Result<()> {
    // IP_BOUND_IF is 25, IPV6_BOUND_IF is 125 on macOS
    let opt = if is_v6 { 125 } else { 25 };
    let level = if is_v6 {
        libc::IPPROTO_IPV6
    } else {
        libc::IPPROTO_IP
    };

    let ret = unsafe {
        libc::setsockopt(
            fd,
            level,
            opt,
            &interface_index as *const _ as *const libc::c_void,
            std::mem::size_of::<u32>() as libc::socklen_t,
        )
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

async fn test_ip_latency_icmp(ip: &str) -> Option<u128> {
    if ip.is_empty() {
        return None;
    }

    let is_ipv6 = ip.contains(':');

    #[cfg(target_os = "macos")]
    {
        if let Some(info) = get_physical_interface_info() {
            let cmd = if is_ipv6 { "ping6" } else { "ping" };
            let mut command = Command::new(cmd);
            command.arg("-c").arg("1").arg("-t").arg("2");

            if is_ipv6 {
                command.arg("-I").arg(&info.name);
            } else {
                // Use -b to bind to interface on macOS
                command.arg("-b").arg(&info.name);
            }
            command.arg(ip);

            match timeout(Duration::from_secs(3), command.output()).await {
                Ok(Ok(output)) if output.status.success() => {
                    let s = String::from_utf8_lossy(&output.stdout);
                    if let Some(time_idx) = s.find("time=") {
                        let after_time = &s[time_idx + 5..];
                        if let Some(space_idx) = after_time.find(' ') {
                            let time_str = &after_time[..space_idx];
                            if let Ok(ms) = time_str.parse::<f64>() {
                                return Some(ms.round() as u128);
                            }
                        }
                    }
                }
                Ok(Ok(output)) => {
                    debug!(
                        "ICMP Ping failed for {} with status: {}. Stderr: {}",
                        ip,
                        output.status,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                Ok(Err(e)) => {
                    debug!("Failed to execute ping for {}: {}", ip, e);
                }
                Err(_) => {
                    debug!("ICMP Ping timed out for {}", ip);
                }
            }
        }
    }

    None
}

async fn test_ip_latency_tcp(ip: &str) -> Option<u128> {
    if ip.is_empty() {
        return None;
    }

    let is_ipv6 = ip.contains(':');
    let addr_str = if is_ipv6 {
        format!("[{}]:443", ip)
    } else {
        format!("{}:443", ip)
    };

    let socket_addr: std::net::SocketAddr = addr_str.parse().ok()?;
    let start = Instant::now();

    let connect_fut = async {
        let socket = if is_ipv6 {
            TcpSocket::new_v6().ok()?
        } else {
            TcpSocket::new_v4().ok()?
        };

        #[cfg(target_os = "macos")]
        {
            if let Some(info) = get_physical_interface_info() {
                let fd = socket.as_raw_fd();
                if let Err(e) = bind_socket_to_interface(fd, info.index, is_ipv6) {
                    debug!("Failed to bind speedtest socket: {}", e);
                }
            }
        }

        timeout(Duration::from_secs(2), socket.connect(socket_addr))
            .await
            .ok()?
            .ok()
    };

    match connect_fut.await {
        Some(_) => Some(start.elapsed().as_millis()),
        _ => None,
    }
}

async fn test_ip_latency(ip: &str) -> Option<u128> {
    // Try ICMP first
    if let Some(lat) = test_ip_latency_icmp(ip).await {
        debug!("Latency for {} (ICMP): {}ms", ip, lat);
        return Some(lat);
    }
    // Fallback to TCP 443
    if let Some(lat) = test_ip_latency_tcp(ip).await {
        debug!("Latency for {} (TCP): {}ms", ip, lat);
        return Some(lat);
    }
    None
}

pub async fn measure_latency(server: &ServerInfo) -> SpeedTestResult {
    let (l4, l6) = tokio::join!(test_ip_latency(&server.ip4), test_ip_latency(&server.ip6));

    SpeedTestResult {
        id: server.id.clone(),
        location: server.location.clone(),
        ip4: server.ip4.clone(),
        ip6: server.ip6.clone(),
        latency4_ms: l4,
        latency6_ms: l6,
    }
}

pub async fn run_speed_test() -> Vec<SpeedTestResult> {
    let servers = get_servers();

    #[cfg(target_os = "macos")]
    let phys_info = get_physical_interface_info();

    #[cfg(target_os = "macos")]
    if let Some(info) = &phys_info {
        debug!(
            "Adding temporary bypass routes for speedtest via {}",
            info.name
        );
        manage_bypass_routes(&servers, info, true);
    }

    let mut tasks = Vec::new();
    for server in servers.clone() {
        tasks.push(tokio::spawn(async move { measure_latency(&server).await }));
    }

    let mut results = Vec::new();
    for task in tasks {
        if let Ok(res) = task.await {
            results.push(res);
        }
    }

    #[cfg(target_os = "macos")]
    if let Some(info) = &phys_info {
        debug!("Removing temporary bypass routes");
        manage_bypass_routes(&servers, info, false);
    }

    results
}
