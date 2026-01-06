use actix_web::{HttpResponse, Responder};
use log::info;
use serde::Serialize;
use std::process::Command;

#[derive(Serialize, Debug)]
struct Interface {
    name: String,
    up: bool,
}

#[derive(Serialize)]
struct ApiResp<T> {
    success: bool,
    data: T,
}

async fn get_interfaces() -> impl Responder {
    info!("Fetching network interfaces");
    let script = "
        [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; 
       (Get-NetRoute -DestinationPrefix '0.0.0.0/0' | Sort-Object RouteMetric)[0] | Get-NetIPInterface | Select-Object -ExpandProperty InterfaceAlias
    ";

    let output = Command::new("powershell")
        .args(["-Command", script])
        .output()
        .expect("Failed to execute command");

    let cmd_output = String::from_utf8_lossy(&output.stdout).trim().to_string();
    HttpResponse::Ok().json(ApiResp {
        success: true,
        data: cmd_output,
    })
}
