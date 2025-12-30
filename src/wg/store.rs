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
