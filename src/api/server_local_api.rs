//! HTTP handlers exposed when the binary is launched in **server** mode
//! (`itunnel -s`). Server lifecycle and TUN logic are delegated to
//! [`crate::wg::server`]; this module is thin glue that:
//!
//! 1. Mutates the in-memory peer list under the global [`WireGuardState`].
//! 2. Persists changes to `~/.itunnel/itunnel_peers.json`.
//! 3. Hot-reloads the running tunnel via `wg::server::apply_peers`.
//! 4. Exposes start/stop/status to the WebUI.

use crate::wg::config::{ConnectionStatus, Peer, Protocol, WireGuardState};
use crate::wg::{server as wg_server, store};
use actix_web::{get, post, web, HttpResponse, Responder};
use base64::{engine::general_purpose, Engine as _};
use log::{error, info};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use x25519_dalek::{PublicKey, StaticSecret};

#[derive(Deserialize, Debug)]
pub struct AddPeerRequest {
    pub name: String,
    pub private_key: Option<String>,
    pub public_key: String,
    pub preshared_key: Option<String>,
    pub allowed_ips: String,
    pub endpoint: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct DeletePeerRequest {
    pub public_key: String,
}

#[derive(Deserialize, Debug)]
pub struct UpdatePeerRequest {
    pub original_public_key: String,
    pub name: String,
    pub private_key: Option<String>,
    pub public_key: String,
    pub preshared_key: Option<String>,
    pub allowed_ips: String,
    pub endpoint: Option<String>,
}

#[derive(Serialize)]
pub struct GeneratePeerResponse {
    pub private_key: String,
    pub public_key: String,
    pub preshared_key: String,
    pub recommended_ip: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct StartServerRequest {
    pub protocol_obfuscation: Option<bool>,
    pub use_tcp: Option<bool>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_allowed_ips(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Mutate the peer list in-memory + on disk + (if running) on the live tunnel.
fn commit_peers(state: &mut WireGuardState, peers: Vec<Peer>) -> Result<(), String> {
    if let Err(e) = store::save_peers(&peers) {
        error!("Failed to persist peers: {}", e);
    }
    wg_server::apply_peers(state, &peers)
}

/// Read server identity (`PrivateKey` and optional `Endpoint`) from `.env`.
/// Returns the corresponding base64 public key on success.
fn server_identity_from_env() -> (String, String) {
    let mut server_pub = String::new();
    let mut server_endpoint = String::new();

    let content = match std::fs::read_to_string(".env") {
        Ok(c) => c,
        Err(_) => return (server_pub, server_endpoint),
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("PrivateKey=") {
            let priv_b64 = rest.trim().trim_matches('"').trim_matches('\'');
            if let Ok(priv_bytes) = general_purpose::STANDARD.decode(priv_b64) {
                if priv_bytes.len() == 32 {
                    let mut bytes = [0u8; 32];
                    bytes.copy_from_slice(&priv_bytes);
                    let secret = StaticSecret::from(bytes);
                    let public = PublicKey::from(&secret);
                    server_pub = general_purpose::STANDARD.encode(public.as_bytes());
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("Endpoint=") {
            server_endpoint = rest.trim().trim_matches('"').trim_matches('\'').to_string();
        }
    }

    (server_pub, server_endpoint)
}

// ---------------------------------------------------------------------------
// Peer CRUD
// ---------------------------------------------------------------------------

#[get("/api/peer_list")]
pub async fn list_peers_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let peers = {
        let state_lock = state.lock().unwrap();
        if let Some(wg) = &state_lock.wg_config {
            wg.peers.clone()
        } else {
            store::load_peers().unwrap_or_default()
        }
    };

    let (server_public_key, server_endpoint) = server_identity_from_env();

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": peers,
        "server_public_key": server_public_key,
        "server_endpoint": server_endpoint,
    }))
}

#[get("/api/peer_generate")]
pub async fn generate_peer_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let mut os_rng = rand::rngs::OsRng;
    let secret = StaticSecret::random_from_rng(&mut os_rng);
    let public = PublicKey::from(&secret);

    let priv_b64 = general_purpose::STANDARD.encode(secret.to_bytes());
    let pub_b64 = general_purpose::STANDARD.encode(public.as_bytes());

    let mut psk_bytes = [0u8; 32];
    os_rng.fill_bytes(&mut psk_bytes);
    let psk_b64 = general_purpose::STANDARD.encode(psk_bytes);

    // Recommend the next free IP in the server's /24. We accept either the
    // legacy 10.99.0.x range or the new default 10.88.0.x and prefer whichever
    // matches the current interface address.
    let (prefix, mut max_octet) = {
        let state_lock = state.lock().unwrap();
        let prefix = state_lock
            .wg_config
            .as_ref()
            .map(|w| w.interface.address.clone())
            .unwrap_or_default()
            .split('.')
            .take(3)
            .collect::<Vec<_>>()
            .join(".");
        let prefix = if prefix.is_empty() {
            "10.88.0".to_string()
        } else {
            prefix
        };

        let mut max_octet = 1u32;
        if let Some(wg) = &state_lock.wg_config {
            for peer in &wg.peers {
                for ip in &peer.allowed_ips {
                    let ip_no_cidr = ip.split('/').next().unwrap_or("");
                    if let Some(rest) = ip_no_cidr.strip_prefix(&format!("{}.", prefix)) {
                        if let Ok(last) = rest.parse::<u32>() {
                            if last > max_octet {
                                max_octet = last;
                            }
                        }
                    }
                }
            }
        }
        (prefix, max_octet)
    };

    max_octet += 1;
    let next_ip = format!("{}.{}/32", prefix, max_octet);

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": GeneratePeerResponse {
            private_key: priv_b64,
            public_key: pub_b64,
            preshared_key: psk_b64,
            recommended_ip: next_ip,
        }
    }))
}

#[post("/api/peer_add")]
pub async fn add_peer_handler(
    req: web::Json<AddPeerRequest>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    let new_peer = Peer {
        name: Some(req.name.clone()),
        private_key: req.private_key.clone(),
        public_key: req.public_key.clone(),
        preshared_key: req.preshared_key.clone().unwrap_or_default(),
        allowed_ips: parse_allowed_ips(&req.allowed_ips),
        endpoint: req.endpoint.clone().unwrap_or_default(),
        persistent_keepalive: Some(25),
    };

    let mut state_lock = state.lock().unwrap();
    let _ = wg_server::load_server_config(&mut state_lock); // ensure wg_config exists

    let mut peers = state_lock
        .wg_config
        .as_ref()
        .map(|w| w.peers.clone())
        .unwrap_or_default();
    peers.push(new_peer);

    match commit_peers(&mut state_lock, peers) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Peer added successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": e
        })),
    }
}

#[post("/api/peer_update")]
pub async fn update_peer_handler(
    req: web::Json<UpdatePeerRequest>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    let mut state_lock = state.lock().unwrap();
    let _ = wg_server::load_server_config(&mut state_lock);

    let mut peers = state_lock
        .wg_config
        .as_ref()
        .map(|w| w.peers.clone())
        .unwrap_or_default();

    let idx = match peers
        .iter()
        .position(|p| p.public_key == req.original_public_key)
    {
        Some(i) => i,
        None => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "success": false,
                "message": "Peer not found"
            }));
        }
    };

    peers[idx].name = Some(req.name.clone());
    peers[idx].private_key = req.private_key.clone();
    peers[idx].public_key = req.public_key.clone();
    peers[idx].preshared_key = req.preshared_key.clone().unwrap_or_default();
    peers[idx].allowed_ips = parse_allowed_ips(&req.allowed_ips);
    peers[idx].endpoint = req.endpoint.clone().unwrap_or_default();

    match commit_peers(&mut state_lock, peers) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Peer updated successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": e
        })),
    }
}

#[post("/api/peer_delete")]
pub async fn delete_peer_handler(
    req: web::Json<DeletePeerRequest>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    let mut state_lock = state.lock().unwrap();
    let _ = wg_server::load_server_config(&mut state_lock);

    let mut peers = state_lock
        .wg_config
        .as_ref()
        .map(|w| w.peers.clone())
        .unwrap_or_default();

    let initial = peers.len();
    peers.retain(|p| p.public_key != req.public_key);
    if peers.len() == initial {
        return HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "Peer not found"
        }));
    }

    match commit_peers(&mut state_lock, peers) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Peer deleted successfully"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": e
        })),
    }
}

// ---------------------------------------------------------------------------
// Lifecycle + status
// ---------------------------------------------------------------------------

#[get("/api/server_status")]
pub async fn server_status_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let state_lock = state.lock().unwrap();
    let is_running =
        state_lock.handle.is_some() && state_lock.status == ConnectionStatus::Connected;

    let (rx_bytes, tx_bytes) = wg_server::stats(&state_lock);

    let peers_count = state_lock
        .wg_config
        .as_ref()
        .map(|wg| wg.peers.len())
        .unwrap_or_else(|| store::load_peers().unwrap_or_default().len());

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "running": is_running,
        "rx_bytes": rx_bytes,
        "tx_bytes": tx_bytes,
        "peers_count": peers_count,
    }))
}

#[post("/api/start")]
pub async fn start_server_handler(
    req: Option<web::Json<StartServerRequest>>,
    state: web::Data<Mutex<WireGuardState>>,
) -> impl Responder {
    let mut state_lock = state.lock().unwrap();
    let protocol_obfuscation = req
        .as_ref()
        .and_then(|r| r.protocol_obfuscation)
        .unwrap_or(false);
    let use_tcp = req.as_ref().and_then(|r| r.use_tcp).unwrap_or(false);
    state_lock.enhance_mode.protocol = if use_tcp { Protocol::TCP } else { Protocol::UDP };
    match wg_server::start(&mut state_lock, protocol_obfuscation) {
        Ok(_) => {
            info!("✅ Server started via API");
            HttpResponse::Ok().json(serde_json::json!({ "success": true }))
        }
        Err(e) => {
            error!("Server start failed: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": e,
            }))
        }
    }
}

#[post("/api/stop")]
pub async fn stop_server_handler(state: web::Data<Mutex<WireGuardState>>) -> impl Responder {
    let mut state_lock = state.lock().unwrap();
    match wg_server::stop(&mut state_lock) {
        Ok(_) => {
            info!("🛑 Server stopped via API");
            HttpResponse::Ok().json(serde_json::json!({ "success": true }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "message": e,
        })),
    }
}

pub fn server_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(generate_peer_handler)
        .service(add_peer_handler)
        .service(update_peer_handler)
        .service(delete_peer_handler)
        .service(list_peers_handler)
        .service(server_status_handler)
        .service(start_server_handler)
        .service(stop_server_handler);
}
