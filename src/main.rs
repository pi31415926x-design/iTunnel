use actix_files::Files;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use itunnel::{interface::get_interfaces, logging};
use std::process::Command;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_opener::OpenerExt;

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

// --- Actix-web 部分 ---

#[get("/")]
async fn hello() -> impl Responder {
    let interface_alias = "get_interface_alias()";
    println!("Retrieved Interface Alias: {}", interface_alias);
    let html_content = format!(
        r#"<!DOCTYPE html>
        <html>
            <head><meta charset="utf-8"></head>
            <body>
                <h1>Tauri Hello</h1>
                <p>你好，这是后台服务</p>
                <p>Interface Alias: {}</p>
            </body>
        </html>"#,
        interface_alias
    );

    HttpResponse::Ok()
        // 关键点：显式设置内容类型和字符集
        .content_type("text/html; charset=utf-8")
        .body(html_content)
}

#[get("/logs")]
async fn get_logs() -> impl Responder {
    let logs = itunnel::logging::get_recent_logs();
    HttpResponse::Ok().json(logs)
}

use itunnel::wg::config::WireGuardState;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

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
            //.service(hello) // 注册路由
            .service(get_interfaces)
            .service(itunnel::wg::config::set_wg_config)
            .service(itunnel::wg::config::get_wg_stats)
            .service(get_logs)
            .service(Files::new("/", static_path.as_ref()).index_file("index.html"))
    })
    .bind(("127.0.0.1", 8181))?
    .run()
    .await
}

// --- Tauri 主程序部分 ---

fn main() {
    /// init logging
    logging::init();

    // Create shared state
    let initial_payload = match itunnel::wg::store::load_config() {
        Ok(Some(json_str)) => {
            match serde_json::from_str::<itunnel::wg::config::WgConfigPayload>(&json_str) {
                Ok(p) => {
                    println!("📄 Loaded config payload from store");
                    Some(p)
                }
                Err(e) => {
                    eprintln!("⚠️ Failed to parse config payload: {}", e);
                    None
                }
            }
        }
        Ok(None) => None,
        Err(e) => {
            eprintln!("⚠️ Failed to load config: {}", e);
            None
        }
    };

    let wg_state = Arc::new(Mutex::new(WireGuardState {
        tun_fd: None,
        handle: None,
        payload: initial_payload,
        status: itunnel::wg::config::ConnectionStatus::Disconnected,
    }));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
                                state.status = itunnel::wg::config::ConnectionStatus::Disconnected;
                            } else {
                                // Connect
                                println!("Tray: Connecting...");
                                // Clone payload to avoid borrow issues
                                if let Some(payload) = state.payload.clone() {
                                    match itunnel::wg::config::apply_wg_config(
                                        &mut *state,
                                        &payload,
                                    ) {
                                        Ok(handle) => {
                                            println!(
                                                "Tray: Connected successfully, handle {}",
                                                handle
                                            );
                                        }
                                        Err(e) => eprintln!("Tray: Failed to connect: {}", e),
                                    }
                                } else {
                                    println!("Tray: No config found. Opening config page.");
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
                            app.exit(0);
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
