use crate::api::remote_api::{SubscribeRequest, SubscriptionPlan};
use crate::speedtest;
use crate::wg::config::{ConnectionStatus, WgConfigPayload, WireGuardState};
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

fn replace_endpoint_ip(input: &str, new_ip: &str) -> String {
    input
        .split_whitespace()
        .map(|token| {
            if let Some(value) = token.strip_prefix("Endpoint=") {
                if let Some((_, port)) = value.split_once(':') {
                    format!("Endpoint={}:{}", new_ip, port)
                } else {
                    token.to_string()
                }
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[post("/api/switch_node")]
pub async fn switch_node_handler(
    req: web::Json<SwitchNodeRequest>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    info!("🔄 Switching to node IP: {}", req.ip);

    // 1. Update the state with the new endpoint
    let handle_res = {
        let mut state_lock = match state.lock() {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to lock state: {}", e);
                return HttpResponse::InternalServerError().body("Internal Server Error");
            }
        };

        if let Some(wg_config) = state_lock.wg_config.as_mut() {
            let port = rand::thread_rng().gen_range(61820..=63820);
            info!("Generated random port: {}", port);
            wg_config.peers[0].endpoint = format!("{}:{}", req.ip, port);
        }

        let new_config = state_lock.json_to_wg_config(state_lock.wg_config.as_ref().unwrap());
        let handle = state_lock.handle.unwrap();

        crate::wg::WireGuardApi::set_config(handle, &new_config)
    };

    match handle_res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "status": "success", "ip": req.ip })),
        Err(e) => {
            error!("Failed to apply config during switch: {}", e);
            HttpResponse::InternalServerError().body("Failed to apply config during switch")
        }
    }
}

#[post("/api/connect")]
pub async fn connect_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    // try to get config from remote
    let (api_client, device_id, is_connected) = {
        let state_lock = state.lock().unwrap();
        (
            state_lock.api_client.clone(),
            state_lock.device_id.clone(),
            state_lock.status == crate::wg::config::ConnectionStatus::Connected,
        )
    };

    if is_connected {
        return HttpResponse::Ok().json(serde_json::json!({ "status": "already_connected" }));
    }

    let config = api_client.get_desktop_cfg(&device_id).await;

    match config {
        Ok(cfg) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&cfg) {
                if let Some(encrypted) = json["wgcfgparam"].as_str() {
                    let mut decoded_str = crate::wg::WireGuardApi::decode_config(encrypted);
                    info!("Decoded config: {}", decoded_str);
                    decoded_str = "[Interface]
PrivateKey = +Bd4l7kpiveFJhF0Tu/Mbg6MocKqUdtj7eUAS3BuDls=
ListenPort = 51820
Jc = 3
Jmin = 10
Jmax = 30
S1 = 11
S2 = 22
H1 = 33
H2 = 44
H3 = 55
H4 = 66
Address = 10.88.0.96/16
DNS = 8.8.8.8, 8.8.4.4
MTU = 1280
[Peer]
PublicKey = qHaIfS7u47/U1AuigBDhOv/p/t6Gy+XKSUdYnPIEKDA=
PresharedKey = rG7z8Z/gkk8wWUA1KwkQ/TS/wFGVBTAEA69igjUhr9Q=
AllowedIPs = 0.0.0.0/1, 128.0.0.0/2, 192.0.0.0/9, 192.128.0.0/11, 192.160.0.0/13, 192.169.0.0/16, 192.170.0.0/15, 192.172.0.0/14, 192.176.0.0/12, 192.192.0.0/10, 193.0.0.0/8, 194.0.0.0/7, 196.0.0.0/6, 200.0.0.0/5, 208.0.0.0/4, 224.0.0.0/3
Endpoint = 52.78.213.238:63685".to_string();
                    let mut state_lock = state.lock().unwrap();
                    state_lock.config = Some(decoded_str);
                    match crate::wg::config::apply_wg_config(&mut *state_lock) {
                        Ok(_) => {
                            state_lock.status = crate::wg::config::ConnectionStatus::Connected;
                        }
                        Err(e) => {
                            error!("Failed to apply config: {}", e);
                        }
                    }
                } else {
                    error!("'gcfgparam' not found in remote config");
                }
            } else {
                error!("Failed to parse remote response as JSON: {}", cfg);
            }
        }
        Err(e) => {
            error!("Failed to get config from remote: {}", e);
            return HttpResponse::InternalServerError().body("Failed to get config from remote");
        }
    }

    HttpResponse::Ok().json(serde_json::json!({ "status": "success" }))
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
