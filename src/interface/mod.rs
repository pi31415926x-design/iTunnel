use actix_web::{get, HttpResponse, Responder};
use log::info;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use serde::Serialize;
pub mod gateway;

#[derive(Serialize)]
struct InterfaceInfo {
    name: String,
    index: u32,
    addr: Vec<String>,
}

#[derive(Serialize)]
struct ApiResp<T> {
    success: bool,
    data: T,
}

#[get("/api/interfaces")]
pub async fn get_interfaces() -> impl Responder {
    info!("Fetching physical network interfaces");

    let interfaces = match NetworkInterface::show() {
        Ok(itfs) => itfs,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ApiResp {
                success: false,
                data: format!("Failed to fetch interfaces: {}", e),
            });
        }
    };

    // Filter for physical interfaces:
    // We use heuristics to exclude common virtual interface names and patterns.
    let physical_interfaces: Vec<InterfaceInfo> = interfaces
        .into_iter()
        .filter(|itf| {
            let name = itf.name.to_lowercase();

            // 1. Exclude loopback
            if name.contains("loopback") || name.contains("lo0") || name == "lo" {
                return false;
            }

            // 2. Exclude common virtual/software interface keywords
            let virtual_keywords = [
                "docker",
                "veth",
                "br-",
                "br0",
                "bridge",
                "tun",
                "tap",
                "vpn",
                "virtual",
                "hyper-v",
                "vbox",
                "vmnet",
                "vmware",
                "utun",
                "wg",
                "tailscale",
                "zerotier",
                "ppp",
                "vEthernet",
            ];

            if virtual_keywords.iter().any(|&kw| name.contains(kw)) {
                return false;
            }

            // 3. Check for MAC address (physical interfaces usually have one)
            // Some virtual interfaces also have MACs, but it's a good secondary check.
            if let Some(mac) = &itf.mac_addr {
                if mac == "00:00:00:00:00:00" {
                    return false;
                }
            } else {
                // If no MAC address is present, it's likely not a physical Ethernet/Wi-Fi interface
                return false;
            }

            true
        })
        .map(|itf| InterfaceInfo {
            name: itf.name,
            index: itf.index,
            addr: itf.addr.into_iter().map(|a| a.ip().to_string()).collect(),
        })
        .collect();

    HttpResponse::Ok().json(ApiResp {
        success: true,
        data: physical_interfaces,
    })
}

/*
// 设置NAT的函数
fn set_nat(interface_alias: &str) -> String {
    let script = format!(
        "New-NetNat -Name 'LAN-NAT' -InternalIPInterfaceAddressPrefix '{}'",
        interface_alias
    );
    let _output = Command::new("powershell")
        .args(["-Command", &script])
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&_output.stdout).trim().to_string()
}

// 设置interface转发启用的函数
fn set_forward_enable(interface_alias: &str) -> String {
    let script = format!(
        "netsh interface ipv4 set interface '{}' forwarding=enabled",
        interface_alias
    );

    let _output = Command::new("powershell")
        .args(["-Command", &script])
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&_output.stdout).trim().to_string()
}

// 获取默认网络接口别名的函数
fn get_interface_alias() -> String {
    let script = "
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
        (Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric)[0] | Get-NetIPInterface | Select-Object -ExpandProperty InterfaceAlias
    ";

    let output = Command::new("powershell")
        .args(["-Command", script])
        .output()
        .expect("Failed to execute command");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

*/
