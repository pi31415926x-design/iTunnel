use crate::api::remote_api::{SubscribeRequest, SubscriptionPlan};
use crate::speedtest;
use crate::wg::config::{ConnectionStatus, Interface, Peer, Wg, WgConfigPayload, WireGuardState};
use crate::wg::WireGuardApi;
use actix_web::{get, post, web, HttpResponse, Responder};
use log::{debug, error, info};
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Serialize)]
pub struct WgStatsResponse {
    pub rx: u64,
    pub tx: u64,
    pub peers: u32,
    pub status: ConnectionStatus,
    pub gateway_enabled: bool,
    pub selected_id: Option<String>,
}

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

#[get("/api/logs")]
pub async fn get_logs() -> impl Responder {
    let logs = crate::logging::get_recent_logs();
    HttpResponse::Ok().json(logs)
}

#[get("/api/user_info")]
pub async fn user_info_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let (api_client, device_id) = {
        let state_lock = state.lock().unwrap();
        (state_lock.api_client.clone(), state_lock.device_id.clone())
    };

    // Re-verify to get the latest expiration from the token/claims
    match api_client.verify_device(&device_id).await {
        Ok(_) => {
            // After verify_device, the token is updated. We can parse it to get exp.
            let token_opt = api_client.token.lock().unwrap().clone();
            if let Some(token) = token_opt {
                if let Ok(claims) = crate::api::jwt::parse_jwt(&token) {
                    let expire_date = chrono::DateTime::from_timestamp(claims.exp, 0)
                        .map(|dt| dt.format("%Y-%m-%d").to_string())
                        .unwrap_or_else(|| "Unknown".to_string());

                    return HttpResponse::Ok().json(serde_json::json!({
                        "device_id": device_id,
                        "expire": expire_date
                    }));
                }
            }
            HttpResponse::Ok().json(serde_json::json!({
                "device_id": device_id,
                "expire": "Expired"
            }))
        }
        Err(e) => HttpResponse::InternalServerError().body(e),
    }
}

#[get("/api/subscribe_plans")]
pub async fn subscribe_plans_handler() -> impl Responder {
    let plans = vec![
        SubscriptionPlan {
            name: "Monthly".to_string(),
            duration: "1 Month".to_string(),
            price: 29.0,
            features: vec![
                "Access to all nodes".to_string(),
                "No speed limit".to_string(),
                "Unlimited traffic".to_string(),
                "Stable connection".to_string(),
            ],
            popular: false,
            cta: "Select monthly".to_string(),
        },
        SubscriptionPlan {
            name: "Quarterly".to_string(),
            duration: "3 Months".to_string(),
            price: 75.0,
            features: vec![
                "Access to all nodes".to_string(),
                "No speed limit".to_string(),
                "Unlimited traffic".to_string(),
                "Stable connection".to_string(),
                "Save 14% vs Monthly".to_string(),
            ],
            popular: false,
            cta: "Select quarterly".to_string(),
        },
        SubscriptionPlan {
            name: "Semi-Annual".to_string(),
            duration: "6 Months".to_string(),
            price: 140.0,
            features: vec![
                "Access to all nodes".to_string(),
                "No speed limit".to_string(),
                "Unlimited traffic".to_string(),
                "Stable connection".to_string(),
                "Save 19% vs Monthly".to_string(),
            ],
            popular: true,
            cta: "Select semi-annual".to_string(),
        },
        SubscriptionPlan {
            name: "Annual".to_string(),
            duration: "1 Year".to_string(),
            price: 260.0,
            features: vec![
                "Access to all nodes".to_string(),
                "No speed limit".to_string(),
                "Unlimited traffic".to_string(),
                "Stable connection".to_string(),
                "Save 25% vs Monthly".to_string(),
                "Priority support".to_string(),
            ],
            popular: false,
            cta: "Select annual".to_string(),
        },
    ];
    HttpResponse::Ok().json(plans)
}

#[get("/api/speedtest")]
pub async fn speed_test_handler() -> impl Responder {
    let results = speedtest::run_speed_test().await;
    HttpResponse::Ok().json(results)
}

#[get("/api/servers")]
pub async fn speed_test_servers_handler() -> impl Responder {
    let servers = speedtest::get_servers();
    HttpResponse::Ok().json(servers)
}

#[derive(Deserialize, Debug)]
pub struct SwitchNodeRequest {
    pub ip: String,
}

#[derive(Deserialize, Debug)]
pub struct AddEndpointRequest {
    pub node_location: String,
    pub node_config: String,
}

#[derive(Deserialize, Debug)]
pub struct UpdateEndpointRequest {
    pub endpoint_id: String,
    pub node_location: String,
    pub node_config: String,
}

#[derive(Deserialize, Debug)]
pub struct DeleteEndpointRequest {
    pub endpoint_id: String,
}

#[derive(Deserialize, Debug)]
pub struct ConnectRequest {
    pub endpoint: String,
}


#[post("/api/switch_node")]
pub async fn switch_node_handler(
    req: web::Json<SwitchNodeRequest>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    info!("🔄 Switching to node IP: {}", req.ip);

    let mut state_lock = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    if state_lock.wg_config.is_none() {
        error!("Cannot switch node: no wg_config available");
        return HttpResponse::BadRequest().body("No WireGuard configuration available");
    }

    // 1. Properly clean up old routes and disconnect if already connected
    // This allows the route to the previous node IP to be cleanly deleted
    if state_lock.status == crate::wg::config::ConnectionStatus::Connected {
        info!("Cleaning up previous connection routes...");
        let _ = state_lock.stop_and_cleanup();
    }

    // 2. Safely apply the new endpoint IP
    if let Some(wg_config) = state_lock.wg_config.as_mut() {
        let port = rand::thread_rng().gen_range(61820..=63820);
        info!("Generated random port for {}: {}", req.ip, port);
        wg_config.peers[0].endpoint = format!("{}:{}", req.ip, port);
    }
    
    // 3. Re-apply networking and wireguard config
    // This adds the new endpoint IP into the primary routing table using the system gateway
    match crate::wg::config::apply_wg_config(&mut state_lock) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "status": "success", "ip": req.ip })),
        Err(e) => {
            error!("Failed to apply config during switch: {}", e);
            HttpResponse::InternalServerError().body(format!("Failed to apply config during switch: {}", e))
        }
    }
}

#[post("/api/connect")]
pub async fn connect_handler(
    req: web::Json<ConnectRequest>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    let endpoint_id = &req.endpoint;
    info!("Connect request for endpoint ID: {}", endpoint_id);

    let mut state_lock = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Internal Server Error"
            }));
        }
    };

    if state_lock.status == crate::wg::config::ConnectionStatus::Connected {
        return HttpResponse::Ok().json(serde_json::json!({
            "status": "already_connected",
            "success": true
        }));
    }

    // Find endpoint by ID
    let endpoint = state_lock
        .available_endpoints
        .iter()
        .find(|e| e.id == *endpoint_id)
        .cloned();

    match endpoint {
        Some(ep) => {
            if let Some(wg_cfg) = ep.wg_config {
                state_lock.wg_config = Some(wg_cfg);
                state_lock.selected_endpoint_id = Some(endpoint_id.clone());

                match crate::wg::config::apply_wg_config(&mut *state_lock) {
                    Ok(_) => {
                        info!("Successfully connected to endpoint: {}", ep.name);
                        HttpResponse::Ok().json(serde_json::json!({
                            "status": "success",
                            "success": true
                        }))
                    }
                    Err(e) => {
                        error!("Failed to apply config: {}", e);
                        HttpResponse::InternalServerError().json(serde_json::json!({
                            "success": false,
                            "message": format!("Failed to apply config: {}", e)
                        }))
                    }
                }
            } else {
                error!("Endpoint has no WireGuard configuration: {}", endpoint_id);
                HttpResponse::BadRequest().json(serde_json::json!({
                    "success": false,
                    "message": "Endpoint has no WireGuard configuration"
                }))
            }
        }
        None => {
            error!("Endpoint not found: {}", endpoint_id);
            HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "message": "Endpoint not found"
            }))
        }
    }
}

#[post("/api/disconnect")]
pub async fn disconnect_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    info!("🛑 Disconnection request received from Overview");
    let mut state_lock = match state.lock() {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    if state_lock.handle.is_some() {
        let _ = state_lock.stop_and_cleanup();
        info!("✅ WireGuard tunnel turned off and routes cleared via API");
    }

    HttpResponse::Ok().json(serde_json::json!({ "status": "success" }))
}

#[get("/api/getwgstats")]
pub async fn get_wg_stats(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let (handle, status, selected_id) = match state.lock() {
        Ok(s) => match s.handle {
            Some(h) => (h, s.status, s.selected_endpoint_id.clone()),
            None => {
                return HttpResponse::Ok().json(WgStatsResponse {
                    rx: 0,
                    tx: 0,
                    peers: 0,
                    status: s.status,
                    gateway_enabled: s.gateway_enabled,
                    selected_id: s.selected_endpoint_id.clone(),
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
                selected_id,
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

    match crate::wg::config::apply_wg_config(&mut state) {
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

    // Filter for physical interfaces
    let physical_interfaces: Vec<InterfaceInfo> = interfaces
        .into_iter()
        .filter(|itf| {
            let name = itf.name.to_lowercase();
            if name.contains("loopback") || name.contains("lo0") || name == "lo" {
                return false;
            }
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
            if let Some(mac) = &itf.mac_addr {
                if mac == "00:00:00:00:00:00" {
                    return false;
                }
            } else {
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

#[post("/api/subscribe_req")]
pub async fn subscribe_req_handler(
    req: web::Json<SubscribeRequest>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    let api_client = {
        let state_lock = match state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to lock state: {}", e);
                return HttpResponse::InternalServerError().body("Internal Server Error");
            }
        };
        state_lock.api_client.clone()
    };

    match api_client.subscribe_req(&req).await {
        Ok(resp) => {
            info!("✅ Subscription Request Forwarded Successfully: {}", resp);
            HttpResponse::Ok().body(resp)
        }
        Err(e) => {
            error!("❌ Failed to forward subscription request: {}", e);
            HttpResponse::InternalServerError().body(e.to_string())
        }
    }
}

#[get("/api/get_wg_config")]
pub async fn get_wg_config(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let (handle, status) = match state.lock() {
        Ok(s) => (s.handle, s.status),
        Err(e) => {
            error!("Failed to lock state: {}", e);
            return HttpResponse::InternalServerError().body("Internal Server Error");
        }
    };

    if let Some(h) = handle {
        if let Some(config) = WireGuardApi::get_config(h) {
            return HttpResponse::Ok().json(serde_json::json!({
                "status": status,
                "config": config
            }));
        }
    }

    HttpResponse::Ok().json(serde_json::json!({
        "status": status,
        "config": null,
        "message": "VPN not connected or config unavailable"
    }))
}

// ========== New API Endpoints for Client Mode ==========

/// GET /api/mode - 获取运行模式（client或server）
#[get("/api/mode")]
pub async fn get_mode_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let state_lock = state.lock().unwrap();
    HttpResponse::Ok().json(serde_json::json!({
        "mode": state_lock.app_mode,
        "success": true
    }))
}

/// GET /api/endpoints - 获取所有可用endpoints（从订阅或手动添加）
#[get("/api/endpoints")]
pub async fn get_endpoints_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let state_lock = state.lock().unwrap();
    let endpoints = &state_lock.available_endpoints;

    HttpResponse::Ok().json(serde_json::json!({
        "endpoints": endpoints,
        "selected_id": state_lock.selected_endpoint_id,
        "success": true
    }))
}

/// POST /api/endpoints/select - 选择endpoint
#[post("/api/endpoints/select")]
pub async fn select_endpoint_handler(
    state: web::Data<Mutex<WireGuardState>>,
    req: web::Json<serde_json::Value>,
) -> impl Responder {
    let endpoint_id = match req.get("endpoint_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "message": "Missing endpoint_id"
            }))
        }
    };

    let mut state_lock = state.lock().unwrap();

    // Find endpoint with matching ID and clone it
    let endpoint = state_lock
        .available_endpoints
        .iter()
        .find(|e| e.id == endpoint_id)
        .cloned();

    match endpoint {
        Some(ep) => {
            state_lock.selected_endpoint_id = Some(endpoint_id);
            info!(
                "Selected endpoint: {} ({}:{})",
                ep.name, ep.address, ep.port
            );

            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": format!("Selected endpoint: {}", ep.name),
                "endpoint": ep
            }))
        }
        None => HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Endpoint not found"
        })),
    }
}

/// Struct for enhance mode settings request
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct EnhanceModeRequest {
    pub protocol: Option<String>, // "tcp" or "udp"
    pub obfuscate: Option<bool>,
    pub obfuscateKey: Option<String>,
    pub proxyMode: Option<String>, // "split" or "global"
}

/// POST /api/settings/enhance-mode - 保存WireGuard增强模式设置
#[post("/api/settings/enhance-mode")]
pub async fn save_enhance_mode_handler(
    state: web::Data<Mutex<WireGuardState>>,
    req: web::Json<EnhanceModeRequest>,
) -> impl Responder {
    let mut state_lock = state.lock().unwrap();

    // Parse protocol
    if let Some(proto_str) = &req.protocol {
        state_lock.enhance_mode.protocol = match proto_str.to_lowercase().as_str() {
            "tcp" => crate::wg::config::Protocol::TCP,
            "udp" => crate::wg::config::Protocol::UDP,
            _ => crate::wg::config::Protocol::UDP,
        };
    }

    // Parse proxy mode
    if let Some(mode_str) = &req.proxyMode {
        state_lock.enhance_mode.proxy_mode = match mode_str.to_lowercase().as_str() {
            "global" => crate::wg::config::ProxyMode::Global,
            "split" => crate::wg::config::ProxyMode::Split,
            _ => crate::wg::config::ProxyMode::Split,
        };
    }

    // Set other options
    if let Some(obfs) = req.obfuscate {
        state_lock.enhance_mode.obfuscate = obfs;
    }

    if let Some(obfs_key) = &req.obfuscateKey {
        state_lock.enhance_mode.obfuscate_key = Some(obfs_key.clone());
    }

    info!(
        "Enhanced mode settings saved: {:?}",
        state_lock.enhance_mode
    );

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Enhanced mode settings saved",
        "enhance_mode": state_lock.enhance_mode
    }))
}

fn parse_wg_config(input: &str) -> Wg {
    let mut interface = Interface::default();
    let mut peers = Vec::new();
    let mut current_peer = None::<Peer>;
    let mut in_interface = false;

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line_lower = line.to_lowercase();
        if line_lower == "[interface]" {
            in_interface = true;
            if let Some(p) = current_peer.take() {
                peers.push(p);
            }
            continue;
        }

        if line_lower == "[peer]" {
            in_interface = false;
            if let Some(p) = current_peer.take() {
                peers.push(p);
            }
            current_peer = Some(Peer::default());
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            if in_interface {
                match key.as_str() {
                    "privatekey" => interface.private_key = value.to_string(),
                    "address" => interface.address = value.to_string(),
                    "dns" => {
                        interface.dns = value.split(',').map(|s| s.trim().to_string()).collect()
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
                        peer.allowed_ips = value.split(',').map(|s| s.trim().to_string()).collect()
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
    }

    if let Some(p) = current_peer {
        peers.push(p);
    }

    Wg { interface, peers }
}

/// GET /api/settings/enhance-mode - 获取当前增强模式设置
#[get("/api/settings/enhance-mode")]
pub async fn get_enhance_mode_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let state_lock = state.lock().unwrap();

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "enhance_mode": state_lock.enhance_mode
    }))
}

/// POST /api/add_endpoint - 添加自定义节点
#[post("/api/add_endpoint")]
pub async fn add_endpoint_handler(
    state: web::Data<Mutex<WireGuardState>>,
    req: web::Json<AddEndpointRequest>,
) -> impl Responder {
    let mut state_lock = state.lock().unwrap();

    info!("Add endpoint request location: {}", req.node_location);

    // Deep parse the configuration
    let wg_config = parse_wg_config(&req.node_config);

    // Extract address/port from the first peer if available
    let mut address = String::new();
    let mut port = 51820;

    if let Some(peer) = wg_config.peers.first() {
        let val = peer.endpoint.trim();
        if let Some(last_colon) = val.rfind(':') {
            let addr_part = &val[..last_colon];
            let port_part = &val[last_colon + 1..];

            address = if addr_part.starts_with('[') && addr_part.ends_with(']') {
                addr_part[1..addr_part.len() - 1].to_string()
            } else {
                addr_part.to_string()
            };

            if let Ok(p) = port_part.parse::<u16>() {
                port = p;
            }
        } else {
            address = val.to_string();
        }
    }

    let new_id = format!("custom-{}", chrono::Utc::now().timestamp());

    let new_endpoint = crate::wg::config::EndpointInfo {
        id: new_id.clone(),
        name: req.node_location.clone(),
        address,
        port,
        location: Some(req.node_location.clone()),
        latency: None,
        from_subscription: false,
        wg_config: Some(wg_config),
    };

    state_lock.available_endpoints.push(new_endpoint);
    info!("New endpoint added: {} - {}", req.node_location, new_id);

    // Save endpoints to disk for persistence
    if let Err(e) = crate::wg::store::save_endpoints(&state_lock.available_endpoints) {
        log::error!("Failed to save endpoints: {}", e);
    }

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Endpoint added successfully",
        "id": new_id
    }))
}

#[post("/api/endpoints/delete")]
pub async fn delete_endpoint_handler(
    state: web::Data<Mutex<WireGuardState>>,
    req: web::Json<DeleteEndpointRequest>,
) -> impl Responder {
    let mut state_lock = state.lock().unwrap();

    let initial_len = state_lock.available_endpoints.len();
    state_lock
        .available_endpoints
        .retain(|e| e.id != req.endpoint_id);

    if state_lock.available_endpoints.len() < initial_len {
        info!("Endpoint deleted: {}", req.endpoint_id);
        if let Err(e) = crate::wg::store::save_endpoints(&state_lock.available_endpoints) {
            log::error!("Failed to save endpoints after delete: {}", e);
        }
        HttpResponse::Ok()
            .json(serde_json::json!({ "success": true, "message": "Endpoint deleted" }))
    } else {
        HttpResponse::NotFound()
            .json(serde_json::json!({ "success": false, "message": "Endpoint not found" }))
    }
}

#[post("/api/endpoints/update")]
pub async fn update_endpoint_handler(
    state: web::Data<Mutex<WireGuardState>>,
    req: web::Json<UpdateEndpointRequest>,
) -> impl Responder {
    let mut state_lock = state.lock().unwrap();

    let id = req.endpoint_id.clone();

    if let Some(endpoint) = state_lock
        .available_endpoints
        .iter_mut()
        .find(|e| e.id == id)
    {
        let wg_config = parse_wg_config(&req.node_config);

        let mut address = String::new();
        let mut port = 51820;

        if let Some(peer) = wg_config.peers.first() {
            let val = peer.endpoint.trim();
            if let Some(last_colon) = val.rfind(':') {
                let addr_part = &val[..last_colon];
                let port_part = &val[last_colon + 1..];

                address = if addr_part.starts_with('[') && addr_part.ends_with(']') {
                    addr_part[1..addr_part.len() - 1].to_string()
                } else {
                    addr_part.to_string()
                };

                if let Ok(p) = port_part.parse::<u16>() {
                    port = p;
                }
            } else {
                address = val.to_string();
            }
        }

        endpoint.name = req.node_location.clone();
        endpoint.location = Some(req.node_location.clone());
        endpoint.address = address;
        endpoint.port = port;
        endpoint.wg_config = Some(wg_config);

        info!("Endpoint updated: {}", id);
        if let Err(e) = crate::wg::store::save_endpoints(&state_lock.available_endpoints) {
            log::error!("Failed to save endpoints after update: {}", e);
        }

        HttpResponse::Ok()
            .json(serde_json::json!({ "success": true, "message": "Endpoint updated" }))
    } else {
        HttpResponse::NotFound()
            .json(serde_json::json!({ "success": false, "message": "Endpoint not found" }))
    }
}
