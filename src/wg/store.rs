use crate::wg::config::EndpointInfo;
use std::fs;
use std::path::PathBuf;

pub fn get_config_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let path = PathBuf::from(home).join(".itunnel").join("wg.conf");
    Some(path)
}

pub fn save_config(config: &str) -> Result<(), String> {
    let path = get_config_path().ok_or("Could not determine config path")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, config).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_config() -> Result<Option<String>, String> {
    let path = get_config_path().ok_or("Could not determine config path")?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(Some(content))
}

pub fn get_endpoints_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let path = PathBuf::from(home).join(".itunnel").join("endpoints.json");
    Some(path)
}

pub fn save_endpoints(endpoints: &[EndpointInfo]) -> Result<(), String> {
    let path = get_endpoints_path().ok_or("Could not determine endpoints path")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(endpoints).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_endpoints() -> Result<Vec<EndpointInfo>, String> {
    let path = get_endpoints_path().ok_or("Could not determine endpoints path")?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let endpoints: Vec<EndpointInfo> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(endpoints)
}

pub fn get_peers_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()?;
    let path = PathBuf::from(home).join(".itunnel").join("itunnel_peers.json");
    Some(path)
}

pub fn save_peers(peers: &[crate::wg::config::Peer]) -> Result<(), String> {
    let path = get_peers_path().ok_or("Could not determine peers path")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(peers).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_peers() -> Result<Vec<crate::wg::config::Peer>, String> {
    let path = get_peers_path().ok_or("Could not determine peers path")?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let peers: Vec<crate::wg::config::Peer> = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(peers)
}
