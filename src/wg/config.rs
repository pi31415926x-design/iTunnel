use log::{error, info};
use serde::{Deserialize, Serialize};
use std::default::Default;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

// ========== Client/Server Mode ==========
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppMode {
    #[serde(rename = "client")]
    Client,
    #[serde(rename = "server")]
    Server,
}

impl Default for AppMode {
    fn default() -> Self {
        AppMode::Client
    }
}

// ========== WireGuard Enhancement Modes ==========
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    #[serde(rename = "udp")]
    UDP,
    #[serde(rename = "tcp")]
    TCP,
}

impl Default for Protocol {
    fn default() -> Self {
        Protocol::UDP
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    #[serde(rename = "split")]
    Split, // 分流模式（默认）
    #[serde(rename = "global")]
    Global, // 全局代理模式
}

impl Default for ProxyMode {
    fn default() -> Self {
        ProxyMode::Split
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhanceMode {
    pub protocol: Protocol,            // TCP/UDP
    pub obfuscate: bool,               // 随机混淆
    pub proxy_mode: ProxyMode,         // 全局/分流
    pub obfuscate_key: Option<String>, // 混淆密钥（可选）
}

impl Default for EnhanceMode {
    fn default() -> Self {
        EnhanceMode {
            protocol: Protocol::UDP,
            obfuscate: false,
            proxy_mode: ProxyMode::Split,
            obfuscate_key: None,
        }
    }
}

// ========== Endpoint Management ==========
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EndpointInfo {
    pub id: String,               // 唯一标识
    pub name: String,             // 节点名称
    pub address: String,          // IP地址或域名
    pub port: u16,                // 端口
    pub location: Option<String>, // 地理位置
    pub latency: Option<u32>,     // 延迟（毫秒）
    pub from_subscription: bool,  // 是否来自订阅
    pub wg_config: Option<Wg>,    // 详细配置
}

impl EndpointInfo {
    pub fn to_string(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AmneziaParams {
    pub jc: u16,
    pub jmin: u16,
    pub jmax: u16,
    pub s1: u16,
    pub s2: u16,
    pub h1: u16,
    pub h2: u16,
    pub h3: u16,
    pub h4: u16,
    pub i1: String,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Interface {
    pub private_key: String,
    pub listen_port: u16,
    pub address: String,
    pub dns: Vec<String>,
    pub mtu: u16,
    pub amnezia_params: Option<AmneziaParams>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Peer {
    pub name: Option<String>,
    pub private_key: Option<String>,
    pub public_key: String,
    pub preshared_key: String,
    pub allowed_ips: Vec<String>,
    pub endpoint: String,
    pub persistent_keepalive: Option<u16>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Wg {
    pub interface: Interface,
    pub peers: Vec<Peer>,
}

#[derive(Debug, Default)]
pub struct WireGuardState {
    // ========== Base Fields ==========
    pub device_id: String,
    pub tun_fd: Option<i32>,
    pub handle: Option<i32>,
    pub payload: Option<WgConfigPayload>,
    pub config: Option<String>,
    pub status: ConnectionStatus,
    pub api_client: crate::api::remote_api::ApiClient,
    pub gateway_enabled: bool,
    pub original_gateway_v4: Option<String>,
    pub original_gateway_v6: Option<String>,
    pub active_endpoint: Option<String>,
    pub wg_config: Option<Wg>,

    // ========== New Fields for Client/Server Mode ==========
    pub app_mode: AppMode,                      // Client or Server
    pub enhance_mode: EnhanceMode,              // TCP/Obfuscate/ProxyMode
    pub available_endpoints: Vec<EndpointInfo>, // Available endpoints (from subscription)
    pub selected_endpoint_id: Option<String>,   // Currently selected endpoint
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

        // 2. Clear network configuration (routes, etc.) — client-mode concern
        if self.app_mode == AppMode::Client {
            if let Err(e) = crate::wg::client::clear_network_config(self) {
                error!("Failed to clear network config: {}", e);
            }
        }

        // 3. Close TUN FD
        if let Some(fd) = self.tun_fd {
            info!("Closing TUN device (FD: {})", fd);
            unsafe { libc::close(fd) };
            self.tun_fd = None;
        }

        // 4. Disable gateway mode if enabled (client-only full-tunnel / NAT path)
        if self.app_mode == AppMode::Client && self.gateway_enabled {
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

        // --- FIX: Ensure Server Mode sets PrivateKey into the struct if empty ---
        if self.app_mode == AppMode::Server && wg.interface.private_key.is_empty() {
            if let Ok(env_content) = std::fs::read_to_string(".env") {
                for line in env_content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("PrivateKey=") {
                        wg.interface.private_key = trimmed.trim_start_matches("PrivateKey=").trim().to_string();
                    }
                    if trimmed.starts_with("ListenPort=") {
                        if let Ok(port) = trimmed.trim_start_matches("ListenPort=").trim().parse::<u16>() {
                            wg.interface.listen_port = port;
                        }
                    }
                }
            }
        }
        
        wg
    }

    // JSON -> wg.conf
    /// 可以用来直接给libwg-go使用。
    ///
    /// Client mode emits AmneziaWG obfuscation when [`EnhanceMode::obfuscate`] is true; server mode
    /// does not use this function for its uapi (see `wg::server::build_uapi`). Server mode also pulls the
    /// PrivateKey from `.env` if missing on the in-memory struct.
    pub fn json_to_wg_config(&self, w: &Wg) -> String {
        let is_server = self.app_mode == AppMode::Server;

        let replace_peers_str = if is_server { "" } else { "replace_peers=true\n" };

        let mut priv_key = w.interface.private_key.clone();
        if is_server && priv_key.is_empty() {
            if let Ok(env_content) = std::fs::read_to_string(".env") {
                for line in env_content.lines() {
                    if let Some(rest) = line.trim().strip_prefix("PrivateKey=") {
                        priv_key = rest.trim().to_string();
                    }
                }
            }
        }

        // Amnezia uapi (client) only when the same Overview / enhance-mode "obfuscation" flag is on.
        let amnezia_block = if is_server {
            String::new()
        } else if self.enhance_mode.obfuscate {
            "jc=3\n\
             jmin=10\n\
             jmax=30\n\
             s1=11\n\
             s2=22\n\
             h1=33\n\
             h2=44\n\
             h3=55\n\
             h4=66\n\
             i1=<b 0x16feff0000000000000001004c01><t><r 28><r 150>\n"
                .to_string()
        } else {
            String::new()
        };

        let mut s = format!(
            "private_key={}\nlisten_port={}\n{}{}",
            Self::base64_to_hex(&priv_key).unwrap_or_default(),
            w.interface.listen_port,
            amnezia_block,
            replace_peers_str
        );

        // TODO: endpoint with port
        // TODO: multiple peers case
        // TODO: multiple allowed_ips case
        for p in &w.peers {
            s += &format!(
                "public_key={}\n",
                Self::base64_to_hex(&p.public_key).unwrap()
            );
            if !p.preshared_key.is_empty() {
                s += &format!(
                    "preshared_key={}\n",
                    Self::base64_to_hex(&p.preshared_key).unwrap()
                );
            }
            for ip in &p.allowed_ips {
                s += &format!("allowed_ip={}\n", ip);
            }
            if !p.endpoint.is_empty() {
                s += &format!("endpoint={}\n", p.endpoint);
            }
        }
        s
    }

    pub fn base64_to_hex(base64_key: &str) -> Result<String, String> {
        use base64::{engine::general_purpose, Engine as _};
        let bytes = general_purpose::STANDARD
            .decode(base64_key)
            .map_err(|e| format!("Failed to decode base64: {}", e))?;
        Ok(bytes.iter().map(|b| format!("{:02x}", b)).collect())
    }
}

/// Parse a `wg.conf`-style INI document into a [`Wg`] struct.
///
/// Unlike [`WireGuardState::wg_config_to_json`], this function is
/// case-insensitive on keys (`PrivateKey`, `privatekey`, `PRIVATEKEY`
/// all work) and tolerates user-pasted snippets with stray whitespace
/// or comments. It does NOT touch `.env` / `WireGuardState`, so it's
/// safe to call from any thread.
pub fn parse_wg_ini(input: &str) -> Wg {
    let mut interface = Interface::default();
    let mut peers: Vec<Peer> = Vec::new();
    let mut current_peer: Option<Peer> = None;
    let mut in_interface = false;

    for raw in input.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        let lower = line.to_lowercase();
        if lower == "[interface]" {
            in_interface = true;
            if let Some(p) = current_peer.take() {
                peers.push(p);
            }
            continue;
        }
        if lower == "[peer]" {
            in_interface = false;
            if let Some(p) = current_peer.take() {
                peers.push(p);
            }
            current_peer = Some(Peer::default());
            continue;
        }

        let (key, value) = match line.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        let key = key.trim().to_lowercase();
        let value = value.trim();

        if in_interface {
            match key.as_str() {
                "privatekey" => interface.private_key = value.to_string(),
                "address" => interface.address = value.to_string(),
                "dns" => {
                    interface.dns = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "mtu" => {
                    if let Ok(m) = value.parse() {
                        interface.mtu = m;
                    }
                }
                "listenport" => {
                    if let Ok(p) = value.parse() {
                        interface.listen_port = p;
                    }
                }
                _ => {}
            }
        } else if let Some(peer) = current_peer.as_mut() {
            match key.as_str() {
                "publickey" => peer.public_key = value.to_string(),
                "presharedkey" => peer.preshared_key = value.to_string(),
                "allowedips" => {
                    peer.allowed_ips = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "endpoint" => peer.endpoint = value.to_string(),
                "persistentkeepalive" => {
                    if let Ok(k) = value.parse() {
                        peer.persistent_keepalive = Some(k);
                    }
                }
                _ => {}
            }
        }
    }

    if let Some(p) = current_peer {
        peers.push(p);
    }

    Wg { interface, peers }
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
        let b64_priv = "yMlj3LbVKMW69kXXh0OpbfZUlEVmkYDao3bk6jTl/EQ=";
        let b64_pub = "qHaIfS7u47/U1AuigBDhOv/p/t6Gy+XKSUdYnPIEKDA=";
        let hex_priv = WireGuardState::base64_to_hex(b64_priv).unwrap();
        let hex_pub = WireGuardState::base64_to_hex(b64_pub).unwrap();

        let state = WireGuardState::default();
        let wg = Wg {
            interface: Interface {
                private_key: b64_priv.into(),
                listen_port: 51820,
                address: "10.0.0.1/24".into(),
                dns: vec!["1.1.1.1".into(), "8.8.8.8".into()],
                mtu: 1420,
                amnezia_params: None,
            },
            peers: vec![Peer {
                name: None,
                private_key: None,
                public_key: b64_pub.into(),
                preshared_key: String::new(),
                allowed_ips: vec!["0.0.0.0/0".into(), "::/0".into()],
                endpoint: "1.2.3.4:51820".into(),
                persistent_keepalive: Some(25),
            }],
        };

        let config = state.json_to_wg_config(&wg);

        println!("test_json_to_wg_config is : {}", config);
        assert!(config.contains(&format!("private_key={}\n", hex_priv)));
        assert!(config.contains("listen_port=51820"));
        assert!(config.contains(&format!("public_key={}\n", hex_pub)));
        assert!(config.contains("allowed_ip=0.0.0.0/0"));
        assert!(config.contains("allowed_ip=::/0"));
        assert!(config.contains("endpoint=1.2.3.4:51820"));
    }
    #[test]
    fn test_base64_to_hex() {
        let b64 = "yMlj3LbVKMW69kXXh0OpbfZUlEVmkYDao3bk6jTl/EQ=";
        let expected_hex = "c8c963dcb6d528c5baf645d78743a96df6549445669180daa376e4ea34e5fc44";
        let result = WireGuardState::base64_to_hex(b64).unwrap();
        assert_eq!(result, expected_hex);
    }
}
