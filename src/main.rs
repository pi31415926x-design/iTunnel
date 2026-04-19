use actix_files::Files;
use actix_web::{web, App, HttpServer};
use itunnel::{api::local_api, logging, wg::config::WireGuardState};
use log::{debug, error, info};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_opener::OpenerExt;

// ========== Parse CLI Arguments ==========
fn parse_cli_args() -> itunnel::wg::config::AppMode {
    let args: Vec<String> = std::env::args().collect();
    
    for arg in &args {
        match arg.as_str() {
            "-s" | "--server" => return itunnel::wg::config::AppMode::Server,
            "-c" | "--client" => return itunnel::wg::config::AppMode::Client,
            _ => {}
        }
    }
    
    // Default to Client mode
    itunnel::wg::config::AppMode::Client
}
async fn start_actix_server(
    static_dir: PathBuf,
    wg_state: Arc<Mutex<WireGuardState>>,
) -> std::io::Result<()> {
    // Wrap the shared state in Actix's Data wrapper
    let wg_data = web::Data::from(wg_state);
    let static_dir = Arc::new(static_dir);

    println!("📁 静态文件目录: {:?}", static_dir);

    HttpServer::new(move || {
        let static_path = static_dir.clone();
        //let cors = actix_cors::Cors::permissive();

        App::new()
            //.wrap(cors)
            .app_data(wg_data.clone())
            .service(local_api::get_interfaces)
            .service(local_api::set_wg_config)
            .service(local_api::get_wg_stats)
            .service(local_api::get_logs)
            .service(local_api::user_info_handler)
            // .service(local_api::get_device_id_handler) // Replaced by user_info_handler
            .service(local_api::subscribe_plans_handler)
            .service(local_api::speed_test_handler)
            .service(local_api::speed_test_servers_handler)
            .service(local_api::enable_gateway_api)
            .service(local_api::disable_gateway_api)
            .service(local_api::gateway_status_api)
            .service(local_api::connect_handler)
            .service(local_api::disconnect_handler)
            .service(local_api::switch_node_handler)
            .service(local_api::subscribe_req_handler)
            .service(local_api::get_wg_config)
            // ========== New Endpoints for Client Mode ==========
            .service(local_api::get_mode_handler)
            .service(local_api::get_endpoints_handler)
            .service(local_api::select_endpoint_handler)
            .service(local_api::add_endpoint_handler)
            .service(local_api::update_endpoint_handler)
            .service(local_api::delete_endpoint_handler)
            .service(local_api::save_enhance_mode_handler)
            .service(local_api::get_enhance_mode_handler)
            .service(Files::new("/", static_path.as_ref()).index_file("index.html"))
    })
    .bind(("127.0.0.1", 8181))?
    .run()
    .await
}

// --- Tauri 主程序部分 ---

fn main() {
    // init logging
    logging::init();

    // ========== Parse CLI Arguments ==========
    let app_mode = parse_cli_args();
    info!("📋 Running in mode: {:?}", app_mode);

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

    let wg_state_run_window = wg_state.clone();
    let wg_state_run_exit = wg_state.clone();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(move |window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                handle_exit(wg_state_run_window.clone());
            }
        })
        .setup(move |app| {
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
            let wg_state_actix = wg_state.clone();
            std::thread::spawn(move || {
                println!("🌐 正在 127.0.0.1:8181 启动 Web 服务...");
                match actix_web::rt::System::new()
                    .block_on(start_actix_server(static_dir, wg_state_actix))
                {
                    Ok(_) => println!("✅ Actix 服务已启动"),
                    Err(e) => {
                        eprintln!("❌ Actix 服务启动失败: {}", e);
                        eprintln!("💡 提示: 请检查端口 8181 是否被占用");
                    }
                }
            });

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
                    move |app, event| match event.id.as_ref() {
                        "connect" => {
                            let mut state = wg_state.lock().unwrap();
                            if let Some(handle) = state.handle {
                                // Disconnect
                                println!("Tray: Disconnecting...");
                                itunnel::wg::WireGuardApi::turn_off(handle);
                                state.handle = None;
                                if let Err(e) =
                                    itunnel::wg::config::clear_network_config(&mut *state)
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
                                    match itunnel::wg::config::apply_wg_config(&mut *state) {
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
                                        .open_url("http://127.0.0.1:8181", None::<&str>);
                                }
                            }
                        }
                        "config" => {
                            let url = "http://127.0.0.1:8181";
                            let _ = app.opener().open_url(url, None::<&str>);
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
        let _ = state.stop_and_cleanup();
    } else {
        log::error!("最终未能获取状态锁，执行强制退出。系统路由可能未能完全恢复！");
    }

    std::process::exit(0);
}

// TODO
// 1. 直接选择节点连接会挂
// 2. 切换节点时, 节点ip不能设置默认官网
