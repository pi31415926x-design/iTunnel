//! Windows: use the GUI subsystem so the built `.exe` does not spawn a console (black) window.
//! Debug builds still use the console for `eprintln!` / `cargo run` logging.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use actix_web::{web, App, HttpServer, HttpResponse};
use rust_embed::RustEmbed;
use itunnel::{api::{local_api, server_local_api}, logging, wg::config::WireGuardState};
use log::{debug, error, info};
use std::{
    net::IpAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri_plugin_opener::OpenerExt;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct EmbeddedAssets;

/// HTTP UI/API server bind. Reads `ListenAddress` and `ListenPort` from `.env` (case-insensitive
/// keys). Defaults: `127.0.0.1:8181` if missing or unparseable.
fn load_http_listen_from_env() -> (IpAddr, u16) {
    const DEFAULT_ADDR: &str = "127.0.0.1";
    const DEFAULT_PORT: u16 = 8181;

    let default_ip: IpAddr = DEFAULT_ADDR.parse().expect("valid default");

    let content = match std::fs::read_to_string(".env") {
        Ok(c) => c,
        Err(_) => return (default_ip, DEFAULT_PORT),
    };

    let mut listen_addr: Option<IpAddr> = None;
    let mut listen_port: Option<u16> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let value = value.trim();
        match key.as_str() {
            "listenaddress" => {
                if let Ok(ip) = value.parse() {
                    listen_addr = Some(ip);
                }
            }
            "listenport" => {
                if let Ok(p) = value.parse() {
                    listen_port = Some(p);
                }
            }
            _ => {}
        }
    }

    (
        listen_addr.unwrap_or(default_ip),
        listen_port.unwrap_or(DEFAULT_PORT),
    )
}

/// Base URL to open in a local browser. Unspecified addresses (`0.0.0.0`, `::`) map to loopback.
fn http_url_for_local_browser(addr: IpAddr, port: u16) -> String {
    use std::net::Ipv4Addr;
    let host = match addr {
        IpAddr::V4(a) if a == Ipv4Addr::UNSPECIFIED => "127.0.0.1".to_string(),
        IpAddr::V4(a) => a.to_string(),
        IpAddr::V6(a) if a.is_unspecified() => "127.0.0.1".to_string(),
        IpAddr::V6(a) => format!("[{}]", a),
    };
    format!("http://{}:{}", host, port)
}

// ========== Parse CLI Arguments ==========
struct StartupOptions {
    app_mode: itunnel::wg::config::AppMode,
    /// No Tauri window / tray; only Actix. See `parse_startup_options` for how this is set.
    headless: bool,
}

/// Mode is chosen by the last of `-s` / `--server` / `-c` / `--client`. With `--server` or
/// `--client` present, default is no GUI; add `--gui` for the tray+window Tauri app. No mode flag
/// keeps the default: Client + Tauri.
fn parse_startup_options() -> StartupOptions {
    let args: Vec<String> = std::env::args().collect();
    let mut app_mode = itunnel::wg::config::AppMode::Client;
    let mut has_mode_flag = false;
    let mut has_gui = false;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-s" | "--server" => {
                app_mode = itunnel::wg::config::AppMode::Server;
                has_mode_flag = true;
            }
            "-c" | "--client" => {
                app_mode = itunnel::wg::config::AppMode::Client;
                has_mode_flag = true;
            }
            "--gui" => has_gui = true,
            _ => {}
        }
    }

    let headless = has_mode_flag && !has_gui;

    StartupOptions { app_mode, headless }
}

fn spawn_actix_background(
    static_dir: PathBuf,
    wg_state: Arc<Mutex<WireGuardState>>,
    app_mode: itunnel::wg::config::AppMode,
    listen_addr: IpAddr,
    listen_port: u16,
) {
    std::thread::spawn(move || {
        println!(
            "🌐 正在 {}:{} 启动 Web 服务...",
            listen_addr, listen_port
        );
        match actix_web::rt::System::new().block_on(start_actix_server(
            static_dir,
            wg_state,
            app_mode,
            listen_addr,
            listen_port,
        )) {
            Ok(_) => println!("✅ Actix 服务已启动"),
            Err(e) => {
                eprintln!("❌ Actix 服务启动失败: {}", e);
                eprintln!("💡 提示: 请检查端口 {} 是否被占用", listen_port);
            }
        }
    });
}

/// Static root for logging / consistency; SPA is still served from [`EmbeddedAssets`].
fn resolve_static_dir_headless() -> PathBuf {
    if cfg!(dev) {
        return std::env::current_dir()
            .expect("cwd")
            .join("frontend")
            .join("dist");
    }
    let cwd = std::env::current_dir().expect("cwd");
    let from_cwd = cwd.join("frontend").join("dist");
    if from_cwd.exists() {
        return from_cwd;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let next_to_bin = dir.join("frontend").join("dist");
            if next_to_bin.exists() {
                return next_to_bin;
            }
        }
    }
    from_cwd
}

/// Injects the CLI app mode so the SPA can set `server` / `client` before `/api/mode`, matching the
/// binary (same as `WireGuardState::app_mode`). Production UI no longer depends on a successful
/// first `GET /api/mode` to pick ServerOverview vs ClientOverview.
fn embed_index_html_with_mode(
    data: std::borrow::Cow<'static, [u8]>,
    mode: itunnel::wg::config::AppMode,
) -> Vec<u8> {
    let mode_str = match mode {
        itunnel::wg::config::AppMode::Server => "server",
        itunnel::wg::config::AppMode::Client => "client",
    };
    let inject = format!(r#"<script>window.__ITUNNEL_APP_MODE__="{mode_str}";</script>"#);
    let s = String::from_utf8_lossy(&data);
    if s.contains("</head>") {
        s.replace("</head>", &format!("{inject}</head>")).into_bytes()
    } else {
        data.into_owned()
    }
}

async fn start_actix_server(
    static_dir: PathBuf,
    wg_state: Arc<Mutex<WireGuardState>>,
    app_mode: itunnel::wg::config::AppMode,
    listen_addr: IpAddr,
    listen_port: u16,
) -> std::io::Result<()> {
    // Wrap the shared state in Actix's Data wrapper
    let wg_data = web::Data::from(wg_state);
    
    println!("⚙️ 启动基于 Rust-Embed 的单文件模式静态托管");

    HttpServer::new(move || {
        
        let app = App::new()
            .app_data(wg_data.clone())
            .configure(local_api::common_routes);

        let app = match app_mode {
            itunnel::wg::config::AppMode::Client => app.configure(local_api::client_routes),
            itunnel::wg::config::AppMode::Server => app.configure(server_local_api::server_routes),
        };

        app.default_service(
            actix_web::web::get().to(move |req: actix_web::HttpRequest| {
                let mode = app_mode;
                async move {
                let mut path = req.path().trim_start_matches('/');
                if path.is_empty() {
                    path = "index.html";
                }
                
                // Check if file exists in the embedded binary
                if let Some(content) = EmbeddedAssets::get(path) {
                    let mime_type = mime_guess::from_path(path).first_or_octet_stream();
                    let body: Vec<u8> = if path == "index.html" {
                        embed_index_html_with_mode(content.data, mode)
                    } else {
                        content.data.into_owned()
                    };
                    return HttpResponse::Ok()
                        .content_type(mime_type.as_ref())
                        .body(body);
                }
                
                // Fallback to index.html for SPA History routing
                if let Some(content) = EmbeddedAssets::get("index.html") {
                    let body = embed_index_html_with_mode(content.data, mode);
                    return HttpResponse::Ok()
                        .content_type("text/html")
                        .body(body);
                }
                
                // Worst case
                HttpResponse::NotFound().body("404 Not Found")
                }
            })
        )
    })
    .bind((listen_addr, listen_port))?
    .run()
    .await
}

// --- Tauri 主程序部分 ---

fn main() {
    // init logging
    logging::init();

    // Auto-generate .env if not exists
    let env_path = std::path::Path::new(".env");
    if !env_path.exists() {
        let env_content = "PrivateKey=<YOUR_SERVER_PRIVATE_KEY>\n\
InterfaceName=utun88\n\
Endpoint=<YOUR_SERVER_IP>:51820\n";
        if let Err(e) = std::fs::write(env_path, env_content) {
            log::error!("⚠️ Failed to create default .env file: {}", e);
        } else {
            log::info!("✅ Created default .env fallback configuration.");
        }
    }

    let (http_bind_addr, http_bind_port) = load_http_listen_from_env();
    let local_api_url = http_url_for_local_browser(http_bind_addr, http_bind_port);

    // ========== Parse CLI Arguments ==========
    let StartupOptions { app_mode, headless } = parse_startup_options();
    info!("📋 Running in mode: {:?}", app_mode);
    if headless {
        info!(
            "🖥️  Headless (CLI): Tauri UI disabled; API {}",
            local_api_url
        );
    }

    // Create shared state
    let initial_payload = match itunnel::wg::store::load_config() {
        Ok(Some(json_str)) => {
            match serde_json::from_str::<itunnel::wg::config::WgConfigPayload>(&json_str) {
                Ok(p) => {
                    debug!("📄 Loaded config payload from store");
                    Some(p)
                }
                Err(e) => {
                    error!("⚠️ Failed to parse config payload: {}", e);
                    None
                }
            }
        }
        Ok(None) => None,
        Err(e) => {
            error!("⚠️ Failed to load config: {}", e);
            None
        }
    };

    let api_client = itunnel::api::remote_api::ApiClient::new();
    let device_id =
        itunnel::config::machine_id::get_machine_id().unwrap_or_else(|_| "unknown".to_string());

    // Perform initial device verification in a separate task/thread if we want it to be non-blocking,
    // or just let the first request handle it. For now, let's just initialize.

    // Load saved custom endpoints from disk
    let saved_endpoints = itunnel::wg::store::load_endpoints().unwrap_or_else(|e| {
        error!("⚠️ Failed to load custom endpoints: {}", e);
        vec![]
    });

    let wg_state = Arc::new(Mutex::new(WireGuardState {
        device_id,
        tun_fd: None,
        handle: None,
        payload: initial_payload,
        status: itunnel::wg::config::ConnectionStatus::Disconnected,
        api_client,
        gateway_enabled: false,
        active_endpoint: None,
        app_mode,                           // ========== Set CLI mode ==========
        enhance_mode: Default::default(),   // Default: UDP, no obfuscate, split mode
        available_endpoints: saved_endpoints, // Populate with saved endpoints
        selected_endpoint_id: None,
        ..Default::default()
    }));

    // Initialize global state for logger access
    itunnel::wg::init_wg_state(wg_state.clone());

    // 0. Add signal handler for Ctrl+C (terminal usage)
    let wg_state_sig = wg_state.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            if let Ok(_) = tokio::signal::ctrl_c().await {
                log::info!("收到 Ctrl+C (SIGINT) 信号，正在执行清理并退出...");
                handle_exit(wg_state_sig);
            }
        });
    });

    if headless {
        let static_dir = resolve_static_dir_headless();
        println!("🚀 iTunnel 启动中 (headless)...");
        println!("📂 工作目录: {:?}", std::env::current_dir().unwrap());
        println!("📁 静态文件路径 (参考): {:?}", static_dir);
        println!("✅ 静态目录存在: {}", static_dir.exists());
        spawn_actix_background(
            static_dir,
            wg_state.clone(),
            app_mode,
            http_bind_addr,
            http_bind_port,
        );
        println!("✅ 本地 API: {} — Ctrl+C 退出", local_api_url);
        let (_tx, rx) = std::sync::mpsc::channel::<()>();
        let _ = rx.recv();
        return;
    }

    let wg_state_run_window = wg_state.clone();
    let wg_state_run_exit = wg_state.clone();
    let open_api_url = local_api_url.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                handle_exit(wg_state_run_window.clone());
            }
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            // 获取静态文件目录路径
            let static_dir = if cfg!(dev) {
                // 开发环境：使用相对路径
                std::env::current_dir()
                    .unwrap()
                    .join("frontend")
                    .join("dist")
            } else {
                // 生产环境：使用 Tauri 资源路径
                app.path()
                    .resource_dir()
                    .expect("无法获取资源目录")
                    .join("frontend")
                    .join("dist")
            };

            println!("🚀 iTunnel 启动中...");
            println!("📂 工作目录: {:?}", std::env::current_dir().unwrap());
            println!("📁 静态文件路径: {:?}", static_dir);
            println!("✅ 静态文件目录存在: {}", static_dir.exists());

            // 1. 在异步运行时中启动 Actix-web
            spawn_actix_background(
                static_dir,
                wg_state.clone(),
                app_mode,
                http_bind_addr,
                http_bind_port,
            );

            // 2. 创建托盘菜单项
            let connect_i = MenuItem::with_id(app, "connect", "连接", true, None::<&str>)?;
            let config_i = MenuItem::with_id(app, "config", "配置", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&connect_i, &config_i, &quit_i])?;

            // 2. 加载图标
            let icon_default_ref = app.default_window_icon().expect("No default window icon");
            let icon_default = tauri::image::Image::new_owned(
                icon_default_ref.rgba().to_vec(),
                icon_default_ref.width(),
                icon_default_ref.height(),
            );

            // Try to load icons from resources
            let res_dir = app.path().resource_dir().unwrap_or_default();
            let icons_path = if cfg!(dev) {
                std::env::current_dir()
                    .unwrap()
                    .join("src-tauri")
                    .join("resources")
                    .join("icons")
            } else {
                res_dir.join("resources").join("icons")
            };

            println!("🎨 Icons path: {:?}", icons_path);

            let icon_gray = tauri::image::Image::from_path(icons_path.join("icon_gray.png"))
                .unwrap_or_else(|e| {
                    eprintln!("⚠️ Failed to load gray icon: {}, using default", e);
                    icon_default.clone()
                });
            let icon_red = tauri::image::Image::from_path(icons_path.join("icon_red.png"))
                .unwrap_or_else(|e| {
                    eprintln!("⚠️ Failed to load red icon: {}, using default", e);
                    icon_default.clone()
                });
            let icon_connected =
                tauri::image::Image::from_path(icons_path.join("icon_connected.png"))
                    .unwrap_or_else(|_| icon_default.clone());

            // 3. 构建托盘
            let _tray = TrayIconBuilder::new()
                .icon(icon_gray.clone()) // Use the loaded gray icon as default
                .menu(&menu)
                .on_menu_event({
                    let wg_state = wg_state.clone();
                    let open_api_url = open_api_url.clone();
                    move |app, event| match event.id.as_ref() {
                        "connect" => {
                            let mut state = wg_state.lock().unwrap();
                            if let Some(handle) = state.handle {
                                // Disconnect
                                println!("Tray: Disconnecting...");
                                itunnel::wg::WireGuardApi::turn_off(handle);
                                state.handle = None;
                                if let Err(e) =
                                    itunnel::wg::client::clear_network_config(&mut *state)
                                {
                                    eprintln!("Tray: Failed to clear network config: {}", e);
                                }
                                if state.gateway_enabled {
                                    let _ = itunnel::interface::gateway::disable_gateway();
                                    state.gateway_enabled = false;
                                }
                                state.status = itunnel::wg::config::ConnectionStatus::Disconnected;
                            } else {
                                // Connect
                                println!("Tray: Connecting...");
                                // try to get config from remote
                                let device_id = state.device_id.clone();
                                let config_res = tauri::async_runtime::block_on(
                                    state.api_client.get_desktop_cfg(&device_id),
                                );

                                if let Ok(cfg) = config_res {
                                    if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(&cfg)
                                    {
                                        if let Some(encrypted) = json["wgcfgparam"].as_str() {
                                            let decoded_str =
                                                itunnel::wg::WireGuardApi::decode_config(encrypted);
                                            state.config = Some(decoded_str);
                                        }
                                    }
                                }

                                if state.payload.is_some() || state.config.is_some() {
                                    match itunnel::wg::client::apply_wg_config(&mut *state) {
                                        Ok(handle) => {
                                            println!(
                                                "Tray: Connected successfully, handle {}",
                                                handle
                                            );
                                        }
                                        Err(e) => eprintln!("Tray: Failed to connect: {}", e),
                                    }
                                } else {
                                    error!("Tray: No config found. Opening config page.");
                                    let _ = app
                                        .opener()
                                        .open_url(&open_api_url, None::<&str>);
                                }
                            }
                        }
                        "config" => {
                            let _ = app.opener().open_url(&open_api_url, None::<&str>);
                        }
                        "quit" => {
                            handle_exit(wg_state.clone());
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // 4. Polling loop to update Tray Menu Text and Icon
            let wg_state_poll = wg_state.clone();
            let connect_i_clone = connect_i.clone();
            let tray_handle = _tray;

            let icon_gray_poll = icon_gray.clone();
            let icon_red_poll = icon_red.clone();
            let icon_connected_poll = icon_connected.clone();

            std::thread::spawn(move || {
                let mut last_status = None;
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if let Ok(state) = wg_state_poll.lock() {
                        let current_status = state.status;

                        // Update text and icon only if status changed
                        if Some(current_status) != last_status {
                            let text = match current_status {
                                itunnel::wg::config::ConnectionStatus::Connected => "断开连接",
                                itunnel::wg::config::ConnectionStatus::Connecting => "正在连接...",
                                _ => "连接",
                            };
                            let _ = connect_i_clone.set_text(text);

                            let icon = match current_status {
                                itunnel::wg::config::ConnectionStatus::Connected => {
                                    &icon_connected_poll
                                }
                                itunnel::wg::config::ConnectionStatus::Connecting => {
                                    &icon_connected_poll
                                }
                                itunnel::wg::config::ConnectionStatus::Error => &icon_red_poll,
                                itunnel::wg::config::ConnectionStatus::Disconnected => {
                                    &icon_gray_poll
                                }
                            };
                            let _ = tray_handle.set_icon(Some(icon.clone()));

                            last_status = Some(current_status);
                        }
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app_handle, event| match event {
            tauri::RunEvent::ExitRequested { .. } => {
                log::info!("📥 Exit requested, cleaning up...");
                handle_exit(wg_state_run_exit.clone());
            }
            _ => {}
        });
}

/// 统一的退出处理函数，确保资源清理
fn handle_exit(wg_state: std::sync::Arc<std::sync::Mutex<itunnel::wg::config::WireGuardState>>) {
    // 尝试获取锁，带重试机制以处理正在进行的连接操作
    let mut state_guard = None;
    for i in 0..6 {
        match wg_state.try_lock() {
            Ok(guard) => {
                state_guard = Some(guard);
                break;
            }
            Err(_) => {
                if i < 5 {
                    log::warn!("正在等待状态锁以执行清理 (尝试 {}/5)...", i + 1);
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    }

    if let Some(mut state) = state_guard {
        match state.app_mode {
            itunnel::wg::config::AppMode::Server => {
                if let Err(e) = itunnel::wg::server::stop(&mut state) {
                    log::error!("Server stop failed during exit: {}", e);
                }
            }
            itunnel::wg::config::AppMode::Client => {
                let _ = state.stop_and_cleanup();
            }
        }
    } else {
        log::error!("最终未能获取状态锁，执行强制退出。系统路由可能未能完全恢复！");
    }

    std::process::exit(0);
}

// TODO
// 1. 直接选择节点连接会挂
// 2. 切换节点时, 节点ip不能设置默认官网
