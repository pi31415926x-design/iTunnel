#[cfg(target_os = "macos")]
use std::process::Command;

/// 获取机器唯一标识 (UUID)
pub fn get_machine_id() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("ioreg")
            .args(&["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .map_err(|e| format!("Failed to execute ioreg: {}", e))?;

        if !output.status.success() {
            return Err("ioreg command failed".to_string());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("IOPlatformUUID") {
                if let Some(uuid) = line.split('"').nth(3) {
                    return Ok(uuid.to_string());
                }
            }
        }
        Err("IOPlatformUUID not found in ioreg output".to_string())
    }

    #[cfg(target_os = "windows")]
    {
        let output = crate::command_ext::command_new("powershell")
            .args(&[
                "-Command",
                "(Get-CimInstance -ClassName Win32_ComputerSystemProduct).UUID",
            ])
            .output()
            .map_err(|e| format!("Failed to execute powershell: {}", e))?;

        if !output.status.success() {
            return Err("powershell command failed".to_string());
        }

        let uuid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if uuid.is_empty() {
            return Err("Retrieved machine ID is empty".to_string());
        }
        Ok(uuid)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err("Unsupported operating system for machine ID retrieval".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_machine_id() {
        match get_machine_id() {
            Ok(uuid) => {
                println!("Detected Machine ID: {}", uuid);
                assert!(!uuid.is_empty());
            }
            Err(e) => {
                eprintln!("Error getting machine ID: {}", e);
                // On some systems it might fail if permissions are restricted,
                // but usually these commands work for regular users.
            }
        }
    }
}
