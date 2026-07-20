use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduledDeletion {
    pub file_path: String,
    pub delete_at: u64,
}

fn get_registry_path(app: &AppHandle) -> PathBuf {
    let mut path = app
        .path()
        .app_data_dir()
        .expect("Failed to resolve app_data_dir");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path.push("scheduled_deletions.json");
    path
}

fn load_registry(app: &AppHandle) -> Vec<ScheduledDeletion> {
    let path = get_registry_path(app);
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(registry) = serde_json::from_str(&content) {
            return registry;
        }
    }
    Vec::new()
}

fn save_registry(app: &AppHandle, registry: &[ScheduledDeletion]) {
    let path = get_registry_path(app);
    if let Ok(json) = serde_json::to_string_pretty(registry) {
        let _ = fs::write(path, json);
    }
}

pub fn schedule_deletion(app: &AppHandle, target_path: &Path, delay_days: u32) {
    let mut registry = load_registry(app);
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
        
    let delete_at = now + (delay_days as u64 * 24 * 60 * 60);

    registry.push(ScheduledDeletion {
        file_path: target_path.to_string_lossy().to_string(),
        delete_at,
    });

    save_registry(app, &registry);
    println!("Scheduled deletion for {:?} in {} days", target_path, delay_days);
}

pub fn sweep_deletions(app: &AppHandle) {
    let mut registry = load_registry(app);
    let mut modified = false;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    registry.retain(|entry| {
        if now >= entry.delete_at {
            let path = PathBuf::from(&entry.file_path);
            if path.exists() {
                if let Err(e) = fs::remove_file(&path) {
                    eprintln!("Scheduled delete failed for {:?}: {}", path, e);
                } else {
                    println!("Scheduled deletion completed for {:?}", path);
                }
            } else {
                println!("Scheduled deletion target not found (already deleted): {:?}", path);
            }
            modified = true;
            false // Remove from registry
        } else {
            true // Keep in registry
        }
    });

    if modified {
        save_registry(app, &registry);
    }
}
