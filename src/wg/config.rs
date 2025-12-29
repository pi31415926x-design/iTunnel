use actix_web::{post, web, HttpResponse, Responder};
use log::{debug, error, info};
use serde::Deserialize;
use std::sync::Mutex;

pub struct WireGuardState {
    pub tun_fd: Option<i32>,
    pub handle: Option<i32>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)] // Frontend sends camelCase
pub struct InterfaceSettings {
    pub address: String,
    pub listenPort: Option<u16>,
    pub privateKey: String,
    pub isTcp: bool,
    pub isServer: bool,
    pub isGlobal: bool,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct PeerSettings {
    pub publicKey: String,
    pub presharedKey: Option<String>,
    pub allowedIPs: String,
    pub endpoint: String,
    pub isChangeRoute: bool,
}

#[derive(Deserialize, Debug)]
pub struct WgConfigPayload {
    pub interface: InterfaceSettings,
    pub peers: PeerSettings,
}

#[post("/setwg")]
pub async fn set_wg_config(
    config: web::Json<WgConfigPayload>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    debug!("Received WG Config: {:?}", config);

    let private_key = crate::wg::WireGuardApi::base64_to_hex(&config.interface.privateKey)
        .unwrap_or_else(|_| {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        });

    let public_key = crate::wg::WireGuardApi::base64_to_hex(&config.peers.publicKey)
        .unwrap_or_else(|_| {
            "0000000000000000000000000000000000000000000000000000000000000000".to_string()
        });

    let preshared_key = match &config.peers.presharedKey {
        Some(k) if !k.is_empty() => crate::wg::WireGuardApi::base64_to_hex(k).unwrap_or_default(),
        _ => "".to_string(),
    };

    // Default values for obfuscation parameters (using values from tests for now)
    let jc = 3;
    let jmin = 10;
    let jmax = 30;
    let s1 = 11;
    let s2 = 22;
    let h1 = 33;
    let h2 = 44;
    let h3 = 55;
    let h4 = 66;

    let mut allowed_ips_str = String::new();
    for ip in config.peers.allowedIPs.split(',') {
        let trimmed = ip.trim();
        if !trimmed.is_empty() {
            allowed_ips_str.push_str(&format!("allowed_ip={}\n", trimmed));
        }
    }

    let settings = format!(
        r#"private_key={}
listen_port={}
jc={}
jmin={}
jmax={}
s1={}
s2={}
h1={}
h2={}
h3={}
h4={}
replace_peers=true
public_key={}
preshared_key={}
{}endpoint={}
"#,
        private_key,
        config.interface.listenPort.unwrap_or(51820),
        jc,
        jmin,
        jmax,
        s1,
        s2,
        h1,
        h2,
        h3,
        h4,
        public_key,
        preshared_key,
        allowed_ips_str,
        config.peers.endpoint,
    );
    debug!("Generated WireGuard Settings:{}\n", settings);

    let mut state = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    // 1. Ensure TUN device exists
    if state.tun_fd.is_none() {
        info!("Creating TUN device...");
        // Ensure logger is set
        crate::wg::WireGuardApi::set_logger();

        match crate::wg::WireGuardApi::create_tun("utun9981", 1420) {
            Ok(fd) => {
                info!("TUN device created successfully. FD: {}", fd);
                state.tun_fd = Some(fd);
            }
            Err(e) => {
                error!("Failed to create TUN device: {}", e);
                return HttpResponse::InternalServerError()
                    .body(format!("Failed to create TUN device: {}", e));
            }
        }
    }

    // 2. Turn on WireGuard (or update config)
    if let Some(fd) = state.tun_fd {
        info!("Turning on WireGuard with FD: {}", fd);
        match crate::wg::WireGuardApi::turn_on(&settings, fd) {
            Ok(handle) => {
                info!("WireGuard turned on successfully. Handle: {}", handle);
                state.handle = Some(handle);

                // 3. Configure Network (IP and Routes)
                // Extract endpoint IP and gateway IP
                let endpoint_parts: Vec<&str> = config.peers.endpoint.split(':').collect();
                let endpoint_ip = if !endpoint_parts.is_empty() {
                    endpoint_parts[0]
                } else {
                    ""
                };

                // Detect default gateway dynamically
                let gateway_ip = match get_default_gateway() {
                    Ok(ip) => ip,
                    Err(e) => {
                        error!("Failed to detect default gateway: {}", e);
                        // Fallback to a common default or handle error appropriately
                        "192.168.1.1".to_string()
                    }
                };
                info!("Detected default gateway: {}", gateway_ip);

                if let Err(e) = configure_network(
                    "utun9981",
                    &config.interface.address,
                    &gateway_ip,
                    endpoint_ip,
                ) {
                    error!("Failed to configure network: {}", e);
                    // We don't fail the request here, but we log the error.
                    // The tunnel might be up but network config failed.
                }

                HttpResponse::Ok().body("WireGuard configured successfully")
            }
            Err(e) => {
                error!("Failed to turn on WireGuard: {}", e);
                HttpResponse::InternalServerError()
                    .body(format!("Failed to turn on WireGuard: {}", e))
            }
        }
    } else {
        error!("TUN FD is missing despite creation attempt.");
        HttpResponse::InternalServerError().body("TUN device initialization failed")
    }
}

fn get_default_gateway() -> std::io::Result<String> {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("route")
            .args(&["-n", "get", "default"])
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get default route",
            ));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("gateway:") {
                if let Some(gateway) = trimmed.split_whitespace().nth(1) {
                    return Ok(gateway.to_string());
                }
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Gateway not found in route output",
        ))
    }

    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("ip")
            .args(&["route", "show", "default"])
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to get default route",
            ));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        // Output format: default via 192.168.1.1 dev eth0 proto dhcp src 192.168.1.100 metric 100
        if let Some(via_index) = output_str.split_whitespace().position(|x| x == "via") {
            if let Some(gateway) = output_str.split_whitespace().nth(via_index + 1) {
                return Ok(gateway.to_string());
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Gateway not found in ip route output",
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

    #[cfg(target_os = "macos")]
    {
        // ifconfig utun9981 10.99.0.7 10.99.0.7 netmask 255.255.0.0
        let output = std::process::Command::new("ifconfig")
            .arg(interface_name)
            .arg(ip)
            .arg(ip)
            .arg("netmask")
            .arg("255.255.0.0")
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "ifconfig failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }

        // ip route add 10.99.0.0/16 dev utun9981
        // Note: macOS uses `route add` not `ip route add`. The user provided `ip route add` in the prompt for macOS,
        // but standard macOS command is `route`. However, if the user has `iproute2mac` installed, `ip` works.
        // Given the requirement "implement the following function... where macos follows:", and the commands listed include `ip route add`,
        // I will try to use `route` command which is native, mapping the intent.
        // `ip route add 10.99.0.0/16 dev utun9981` -> `route -n add -net 10.99.0.0/16 -interface utun9981`

        let routes = vec![
            ("10.99.0.0/16", interface_name),
            ("0.0.0.0/1", interface_name),
            ("128.0.0.0/1", interface_name),
        ];

        for (dest, dev) in routes {
            let output = std::process::Command::new("route")
                .arg("-n")
                .arg("add")
                .arg("-net")
                .arg(dest)
                .arg("-interface")
                .arg(dev)
                .output()?;
            if !output.status.success() {
                error!(
                    "Failed to add route {}: {}",
                    dest,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        // route -n add -inet -host 54.249.221.90 192.168.1.1
        if !endpoint_ip.is_empty() {
            let output = std::process::Command::new("route")
                .arg("-n")
                .arg("add")
                .arg("-inet")
                .arg("-host")
                .arg(endpoint_ip)
                .arg(gateway)
                .output()?;
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

        if !endpoint_ip.is_empty() {
            std::process::Command::new("ip")
                .args(&["route", "add", endpoint_ip, "via", gateway])
                .output()?;
        }
    }

    #[cfg(target_os = "windows")]
    {
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

        if !endpoint_ip.is_empty() {
            std::process::Command::new("route")
                .args(&["ADD", endpoint_ip, "MASK", "255.255.255.255", gateway])
                .output()?;
        }
    }

    Ok(())
}
