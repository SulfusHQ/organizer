use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

use crate::rules::{get_matching_rule, Rule};

fn get_unique_path(target_path: &PathBuf) -> PathBuf {
    if !target_path.exists() {
        return target_path.clone();
    }
    let mut counter = 1;
    let stem = target_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    let ext = target_path
        .extension()
        .unwrap_or_default()
        .to_string_lossy();
    let parent = target_path.parent().unwrap();
    loop {
        let new_name = if ext.is_empty() {
            format!("{} ({})", stem, counter)
        } else {
            format!("{} ({}).{}", stem, counter, ext)
        };
        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return new_path;
        }
        counter += 1;
    }
}

pub fn start_watcher(watch_path: PathBuf, rules: Vec<Rule>, app: AppHandle) {
    let active_files = Arc::new(Mutex::new(HashMap::<PathBuf, (Instant, u32)>::new()));
    let active_files_clone = Arc::clone(&active_files);
    let rules_clone = rules.clone();

    // Debounce and Processing Thread
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(500));
            let mut files_to_process = Vec::new();

            if let Ok(mut map) = active_files_clone.lock() {
                let now = Instant::now();
                map.retain(|path, (last_modified, retry_count)| {
                    let delay = match *retry_count {
                        0 => 1500,   // 1.5s
                        1 => 5000,   // 5s
                        2 => 30000,  // 30s
                        _ => 300000, // 5 mins max
                    };

                    if now.duration_since(*last_modified) > Duration::from_millis(delay) {
                        files_to_process.push((path.clone(), *retry_count));
                        false // Remove from map
                    } else {
                        true // Keep in map
                    }
                });
            }

            if files_to_process.is_empty() {
                continue;
            }

            // GC the ignore_list to prevent memory leak
            if let Some(state) = app.try_state::<crate::AppState>() {
                if let Ok(mut ignore_list) = state.ignore_list.lock() {
                    ignore_list.retain(|_, time| {
                        Instant::now().duration_since(*time) < Duration::from_secs(5)
                    });
                }
            }

            // Load fresh rules on each run for hot-reloading (only once per batch!)
            let current_rules = crate::rules::load_rules(&crate::get_rules_path(&app))
                .unwrap_or_else(|_| rules_clone.clone());

            for (file_path, retry_count) in files_to_process {
                // Ignore browser temporary downloads
                if file_path
                    .extension()
                    .map_or(false, |ext| ext == "crdownload" || ext == "part")
                {
                    continue;
                }

                // Check if file was recently undone
                let mut should_ignore = false;
                if let Some(state) = app.try_state::<crate::AppState>() {
                    if let Ok(ignore_list) = state.ignore_list.lock() {
                        if ignore_list.contains_key(&file_path) {
                            should_ignore = true;
                        }
                    }
                }

                if should_ignore {
                    println!("Ignoring recently undone file: {:?}", file_path);
                    continue;
                }

                if let Some(rule) = get_matching_rule(&current_rules, &file_path) {
                    println!(
                        "\nFile matches rule '{}': {:?}",
                        rule.name,
                        file_path.file_name().unwrap()
                    );

                    let mut current_path = file_path.clone();

                    for action in &rule.actions {
                        match action {
                            crate::rules::Action::Move { target_folder } => {
                                let target_dir = PathBuf::from(target_folder);
                                if !target_dir.exists() {
                                    let _ = fs::create_dir_all(&target_dir);
                                }
                                if let Some(file_name) = current_path.file_name() {
                                    let base_target_path = target_dir.join(file_name);
                                    let target_path = get_unique_path(&base_target_path);
                                    println!("Moving file to: {:?}", target_path);

                                    let success = fs::rename(&current_path, &target_path).is_ok()
                                        || (fs::copy(&current_path, &target_path).is_ok()
                                            && fs::remove_file(&current_path).is_ok());

                                    if success {
                                        println!("Move successful!");
                                        crate::history::add_history_entry(
                                            &app,
                                            file_name.to_string_lossy().to_string(),
                                            current_path.to_string_lossy().to_string(),
                                            target_path.to_string_lossy().to_string(),
                                        );
                                        let _ = app.emit("history-updated", ());
                                        current_path = target_path;
                                    } else {
                                        if retry_count < 3 {
                                            eprintln!(
                                                "Failed to move file, backoff retry #{}",
                                                retry_count + 1
                                            );
                                        }
                                        if let Ok(mut map) = active_files_clone.lock() {
                                            map.insert(
                                                file_path.clone(),
                                                (Instant::now(), retry_count + 1),
                                            );
                                        }
                                        break;
                                    }
                                }
                            }
                            crate::rules::Action::Rename { pattern } => {
                                if let (Some(stem), Some(ext)) =
                                    (current_path.file_stem(), current_path.extension())
                                {
                                    let new_name = pattern
                                        .replace("{filename}", &stem.to_string_lossy())
                                        .replace("{ext}", &ext.to_string_lossy());

                                    let mut target_path = current_path.clone();
                                    target_path.set_file_name(new_name);
                                    let target_path = get_unique_path(&target_path);

                                    if fs::rename(&current_path, &target_path).is_ok() {
                                        println!("Rename successful!");
                                        current_path = target_path;
                                    } else {
                                        if retry_count < 3 {
                                            eprintln!(
                                                "Failed to rename file, backoff retry #{}",
                                                retry_count + 1
                                            );
                                        }
                                        if let Ok(mut map) = active_files_clone.lock() {
                                            map.insert(
                                                file_path.clone(),
                                                (Instant::now(), retry_count + 1),
                                            );
                                        }
                                        break;
                                    }
                                }
                            }
                            crate::rules::Action::Delete { delay_days } => {
                                if *delay_days == 0 {
                                    if fs::remove_file(&current_path).is_ok() {
                                        println!("Delete successful!");
                                        break;
                                    } else {
                                        if retry_count < 3 {
                                            eprintln!(
                                                "Failed to delete file, backoff retry #{}",
                                                retry_count + 1
                                            );
                                        }
                                        if let Ok(mut map) = active_files_clone.lock() {
                                            map.insert(
                                                file_path.clone(),
                                                (Instant::now(), retry_count + 1),
                                            );
                                        }
                                        break;
                                    }
                                } else {
                                    crate::scheduler::schedule_deletion(&app, &current_path, *delay_days);
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    println!("No rule matches file: {:?}", file_path.file_name().unwrap());
                }
            }
        }
    });

    // File System Watcher Thread
    thread::spawn(move || {
        let (tx, rx) = channel();
        let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to create watcher: {:?}", e);
                return;
            }
        };

        println!("Started watching directory: {:?}", watch_path);

        if let Err(e) = watcher.watch(&watch_path, RecursiveMode::NonRecursive) {
            eprintln!("Failed to watch directory: {:?}", e);
            return;
        }

        for res in rx {
            if let Ok(event) = res {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) => {
                        for path in event.paths {
                            if path.is_file() {
                                if let Ok(mut map) = active_files.lock() {
                                    if !map.contains_key(&path) {
                                        map.insert(path, (Instant::now(), 0));
                                    } else {
                                        // Update the instant without resetting retry_count
                                        if let Some(entry) = map.get_mut(&path) {
                                            entry.0 = Instant::now();
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });
}
