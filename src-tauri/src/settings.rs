use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub watch_folder: String,
    pub autostart: bool,
}

pub fn get_settings_path(app: &tauri::AppHandle) -> PathBuf {
    let mut path = app.path().app_data_dir().expect("Failed to resolve app_data_dir");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path.push("settings.json");
    path
}

pub fn load_settings(app: &tauri::AppHandle) -> Settings {
    let path = get_settings_path(app);
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(settings) = serde_json::from_str(&content) {
            return settings;
        }
    }
    
    // Default settings
    Settings {
        watch_folder: app.path().download_dir().unwrap_or_default().to_string_lossy().to_string(),
        autostart: true,
    }
}

pub fn save_settings(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let path = get_settings_path(app);
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}
