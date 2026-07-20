mod history;
mod rules;
mod scheduler;
mod settings;
mod watcher;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tauri::Manager;
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    WindowEvent,
};

pub struct AppState {
    pub ignore_list: Mutex<HashMap<PathBuf, Instant>>,
}

pub fn get_rules_path(app: &tauri::AppHandle) -> PathBuf {
    let mut path = app
        .path()
        .app_data_dir()
        .expect("Failed to resolve app_data_dir");
    if !path.exists() {
        let _ = std::fs::create_dir_all(&path);
    }
    path.push("rules.json");
    path
}

#[tauri::command]
fn get_rules(app: tauri::AppHandle) -> Result<Vec<rules::Rule>, String> {
    let rules_path = get_rules_path(&app);
    rules::load_rules(&rules_path)
}

#[tauri::command]
fn save_rules(app: tauri::AppHandle, rules: Vec<rules::Rule>) -> Result<(), String> {
    let path = get_rules_path(&app);
    rules::save_rules(&path, &rules)
}

#[tauri::command]
fn get_history(app: tauri::AppHandle) -> Vec<history::HistoryEntry> {
    history::load_history(&app)
}

#[tauri::command]
fn undo_move(
    id: String,
    state: tauri::State<AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    if let Some(entry) = history::get_entry(&app, &id) {
        if let Ok(mut ignore) = state.ignore_list.lock() {
            ignore.insert(PathBuf::from(&entry.original_path), Instant::now());
        }
    }
    history::undo_entry(&app, &id)
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    let clean_path = path.replace("\\\\", "\\");

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &clean_path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &clean_path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn pick_folder() -> Result<Option<String>, String> {
    if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
        Ok(Some(folder.path().to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> settings::Settings {
    settings::load_settings(&app)
}

#[tauri::command]
fn update_settings(
    app: tauri::AppHandle,
    new_settings: settings::Settings,
) -> Result<(), String> {
    settings::save_settings(&app, &new_settings)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            app.manage(AppState {
                ignore_list: Mutex::new(HashMap::new()),
            });
            let app_handle = app.handle().clone();

            let rules_path = get_rules_path(&app_handle);
            let rules = match rules::load_rules(&rules_path) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Could not load rules: {:?}", e);
                    Vec::new()
                }
            };

            let settings = settings::load_settings(&app_handle);
            let watch_dir = std::path::PathBuf::from(&settings.watch_folder);
            
            if watch_dir.exists() {
                watcher::start_watcher(watch_dir, rules, app_handle.clone());
            } else if let Ok(downloads_dir) = app.path().download_dir() {
                watcher::start_watcher(downloads_dir, rules, app_handle.clone());
            } else {
                eprintln!("Could not find the watch directory.");
            }

            // Spawn Reaper Thread
            let reaper_app_handle = app_handle.clone();
            std::thread::spawn(move || {
                loop {
                    scheduler::sweep_deletions(&reaper_app_handle);
                    // Sleep for 1 hour (3600 seconds)
                    std::thread::sleep(std::time::Duration::from_secs(3600));
                }
            });

            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let open_i = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
            let menu = MenuBuilder::new(app).item(&open_i).item(&quit_i).build()?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "quit" => app.exit(0),
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_rules,
            save_rules,
            get_history,
            undo_move,
            open_folder,
            pick_folder,
            get_settings,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
