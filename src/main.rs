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

use itunnel::wg::config::WireGuardState;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

async fn start_actix_server(static_dir: PathBuf) -> std::io::Result<()> {
    let wg_state = web::Data::new(Mutex::new(WireGuardState {
        tun_fd: None,
        handle: None,
    }));

    let static_dir = Arc::new(static_dir);

    println!("📁 静态文件目录: {:?}", static_dir);

    HttpServer::new(move || {
        let static_path = static_dir.clone();
        App::new()
            .app_data(wg_state.clone())
            //.service(hello) // 注册路由
            .service(get_interfaces)
            .service(itunnel::wg::config::set_wg_config)
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
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
            // 使用 tauri::async_runtime::spawn 确保它不会阻塞主线程（托盘和 UI 线程）
            // Start Actix-web in a dedicated thread because actix-web's server future
            // is not Send and cannot be moved across threads required by tauri's async runtime.
            std::thread::spawn(move || {
                println!("🌐 正在 127.0.0.1:8181 启动 Web 服务...");
                // Create a new Actix system and block on the server future.
                match actix_web::rt::System::new().block_on(start_actix_server(static_dir)) {
                    Ok(_) => println!("✅ Actix 服务已启动"),
                    Err(e) => {
                        eprintln!("❌ Actix 服务启动失败: {}", e);
                        eprintln!("💡 提示: 请检查端口 8181 是否被占用");
                    }
                }
            });

            // 2. 创建托盘菜单项
            let config_i =
                MenuItem::with_id(app, "config", "配置 (打开浏览器)", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "退出程序", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&config_i, &quit_i])?;

            // 3. 构建托盘
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "config" => {
                        let url = "http://127.0.0.1:8181";
                        let _ = app.opener().open_url(url, None::<&str>);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
