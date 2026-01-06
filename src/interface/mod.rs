pub mod gateway;

/*
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

*/
