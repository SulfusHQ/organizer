use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: String,
    pub file_name: String,
    pub original_path: String,
    pub target_path: String,
    pub timestamp: u64,
}

pub fn get_history_path(app: &AppHandle) -> PathBuf {
    let mut path = app
        .path()
        .app_data_dir()
        .expect("Failed to resolve app_data_dir");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path.push("history.json");
    path
}

pub fn load_history(app: &AppHandle) -> Vec<HistoryEntry> {
    let path = get_history_path(app);
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Vec::new()),
        Err(_) => Vec::new(),
    }
}

pub fn get_entry(app: &AppHandle, id: &str) -> Option<HistoryEntry> {
    let history = load_history(app);
    history.into_iter().find(|e| e.id == id)
}

pub fn add_history_entry(
    app: &AppHandle,
    file_name: String,
    original_path: String,
    target_path: String,
) {
    let mut history = load_history(app);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let entry = HistoryEntry {
        id: format!("{}-{}", timestamp, file_name),
        file_name,
        original_path,
        target_path,
        timestamp,
    };

    history.insert(0, entry);
    history.truncate(100);

    save_history(app, &history);
}

fn save_history(app: &AppHandle, history: &[HistoryEntry]) {
    if let Ok(json) = serde_json::to_string_pretty(history) {
        let _ = fs::write(get_history_path(app), json);
    }
}

pub fn undo_entry(app: &AppHandle, id: &str) -> Result<(), String> {
    let mut history = load_history(app);

    if let Some(index) = history.iter().position(|e| e.id == id) {
        let entry = &history[index];
        let original = PathBuf::from(&entry.original_path);
        let target = PathBuf::from(&entry.target_path);

        if target.exists() {
            fs::rename(&target, &original)
                .or_else(|_| fs::copy(&target, &original).and_then(|_| fs::remove_file(&target)))
                .map_err(|e| format!("Failed to move file back: {}", e))?;

            history.remove(index);
            save_history(app, &history);
            Ok(())
        } else {
            Err("File no longer exists at target location".to_string())
        }
    } else {
        Err("Entry not found".to_string())
    }
}
