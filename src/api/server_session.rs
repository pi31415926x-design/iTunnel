//! Server-mode Web UI authentication: cookie sessions (15 minutes) when
//! `login_user` and `login_pwd` are set in `.env`.
//!
//! `GET /api/get_wg_config` from a loopback peer (e.g. `curl http://127.0.0.1:...`) is allowed
//! without a session so local diagnostics still work when login is enabled.

use actix_web::cookie::time::Duration as CookieDuration;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::middleware::Next;
use actix_web::web;
use actix_web::body::BoxBody;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use actix_web::http::Method;
use actix_web::{get, post, HttpRequest, HttpResponse, Responder};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const SESSION_COOKIE_NAME: &str = "itunnel_session";
const SESSION_TTL: Duration = Duration::from_secs(15 * 60);

fn strip_env_value(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn read_login_creds() -> (String, String) {
    let content = std::fs::read_to_string(".env").unwrap_or_default();
    let mut user = String::new();
    let mut pwd = String::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        match k.trim().to_lowercase().as_str() {
            "login_user" => user = strip_env_value(v),
            "login_pwd" => pwd = strip_env_value(v),
            _ => {}
        }
    }
    (user, pwd)
}

#[derive(Clone)]
pub struct ServerSessionState {
    sessions: Arc<Mutex<HashMap<String, Instant>>>,
    pub login_required: bool,
    username: String,
    password: String,
}

impl ServerSessionState {
    pub fn from_env() -> Self {
        let (username, password) = read_login_creds();
        let login_required = !username.is_empty() && !password.is_empty();
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            login_required,
            username,
            password,
        }
    }

    pub fn login_required_from_env() -> bool {
        let (u, p) = read_login_creds();
        !u.is_empty() && !p.is_empty()
    }

    fn prune_expired(map: &mut HashMap<String, Instant>) {
        let now = Instant::now();
        map.retain(|_, exp| now <= *exp);
    }

    fn session_cookie(token: &str) -> Cookie<'static> {
        Cookie::build(SESSION_COOKIE_NAME, token.to_owned())
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .max_age(CookieDuration::seconds(SESSION_TTL.as_secs() as i64))
            .finish()
    }

    fn clear_session_cookie() -> Cookie<'static> {
        let mut c = Cookie::build(SESSION_COOKIE_NAME, "")
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .finish();
        c.make_removal();
        c
    }

    pub fn auth_status(&self, req: &HttpRequest) -> HttpResponse {
        let authenticated = if !self.login_required {
            true
        } else {
            self.validate_session(req).is_ok()
        };
        HttpResponse::Ok().json(serde_json::json!({
            "authenticated": authenticated,
            "login_required": self.login_required,
        }))
    }

    fn validate_session(&self, req: &HttpRequest) -> Result<(), ()> {
        let token = req.cookie(SESSION_COOKIE_NAME).ok_or(())?.value().to_string();
        let mut guard = self.sessions.lock().unwrap();
        Self::prune_expired(&mut guard);
        let exp = guard.get(&token).copied().ok_or(())?;
        if Instant::now() > exp {
            guard.remove(&token);
            return Err(());
        }
        Ok(())
    }

    fn issue_token(&self) -> String {
        let mut raw = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut raw);
        URL_SAFE_NO_PAD.encode(raw)
    }

    pub fn create_session(&self) -> Cookie<'static> {
        let token = self.issue_token();
        let exp = Instant::now() + SESSION_TTL;
        self.sessions
            .lock()
            .unwrap()
            .insert(token.clone(), exp);
        Self::session_cookie(&token)
    }

    pub fn revoke_session(&self, req: &HttpRequest) {
        if let Some(c) = req.cookie(SESSION_COOKIE_NAME) {
            self.sessions.lock().unwrap().remove(c.value());
        }
    }
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[post("/api/login")]
pub async fn login_handler(
    body: web::Json<LoginBody>,
    sessions: web::Data<ServerSessionState>,
) -> impl Responder {
    if !sessions.login_required {
        return HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Login not configured",
        }));
    }
    if body.username == sessions.username && body.password == sessions.password {
        let cookie = sessions.create_session();
        HttpResponse::Ok()
            .cookie(cookie)
            .json(serde_json::json!({ "success": true }))
    } else {
        HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Invalid username or password",
        }))
    }
}

#[post("/api/logout")]
pub async fn logout_handler(
    req: HttpRequest,
    sessions: web::Data<ServerSessionState>,
) -> impl Responder {
    sessions.revoke_session(&req);
    HttpResponse::Ok()
        .cookie(ServerSessionState::clear_session_cookie())
        .json(serde_json::json!({ "success": true }))
}

#[get("/api/auth/status")]
pub async fn auth_status_handler(req: HttpRequest, sessions: web::Data<ServerSessionState>) -> impl Responder {
    sessions.auth_status(&req)
}

fn server_api_public(path: &str) -> bool {
    matches!(
        path,
        "/api/login" | "/api/logout" | "/api/auth/status" | "/api/mode"
    )
}

/// True when the TCP peer is loopback (e.g. `curl http://127.0.0.1:...`). Used to allow
/// local-only GET `/api/get_wg_config` without a session when login is enabled.
fn peer_is_loopback(req: &HttpRequest) -> bool {
    let ci = req.connection_info();
    let Some(peer) = ci.peer_addr() else {
        return false;
    };
    let peer = peer.to_string();
    SocketAddr::from_str(&peer)
        .map(|a| a.ip().is_loopback())
        .unwrap_or_else(|_| {
            peer.starts_with("127.0.0.1:")
                || peer.starts_with("[::1]:")
                || peer == "127.0.0.1"
                || peer == "::1"
        })
}

pub async fn server_auth_middleware(
    req: ServiceRequest,
    next: Next<impl actix_web::body::MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let Some(sessions) = req.app_data::<web::Data<ServerSessionState>>() else {
        return next.call(req).await.map(|r| r.map_into_boxed_body());
    };
    if !sessions.login_required {
        return next.call(req).await.map(|r| r.map_into_boxed_body());
    }
    let path = req.path();
    if !path.starts_with("/api/") {
        return next.call(req).await.map(|r| r.map_into_boxed_body());
    }
    if server_api_public(path) {
        return next.call(req).await.map(|r| r.map_into_boxed_body());
    }
    if path == "/api/get_wg_config"
        && *req.method() == Method::GET
        && peer_is_loopback(req.request())
    {
        return next.call(req).await.map(|r| r.map_into_boxed_body());
    }
    if sessions.validate_session(req.request()).is_ok() {
        return next.call(req).await.map(|r| r.map_into_boxed_body());
    }
    Ok(req
        .into_response(
            HttpResponse::Unauthorized().json(serde_json::json!({
                "success": false,
                "message": "Unauthorized",
            })),
        )
        .map_into_boxed_body())
}

pub fn register(cfg: &mut web::ServiceConfig) {
    cfg.service(login_handler)
        .service(logout_handler)
        .service(auth_status_handler);
}
