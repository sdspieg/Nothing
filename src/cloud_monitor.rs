use crate::file_entry::FileEntry;
use crate::index::FileIndex;
use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Monitors cloud/network folders using filesystem events
pub struct CloudMonitor {
    _watchers: Vec<notify::RecommendedWatcher>,
}

impl CloudMonitor {
    /// Create new cloud monitor for specified folders
    pub fn new(cloud_folders: Vec<PathBuf>, index: Arc<Mutex<FileIndex>>) -> Result<Self> {
        let mut watchers = Vec::new();

        for folder in cloud_folders {
            let index_clone = Arc::clone(&index);

            let (tx, rx) = std::sync::mpsc::channel();

            let mut watcher = notify::recommended_watcher(move |res| {
                if let Err(e) = tx.send(res) {
                    eprintln!("Cloud monitor send error: {}", e);
                }
            })?;

            watcher.watch(&folder, RecursiveMode::Recursive)?;

            // Spawn thread to handle events
            let folder_name = folder.display().to_string();
            thread::spawn(move || {
                println!("📁 Monitoring cloud folder: {}", folder_name);

                for event_result in rx {
                    match event_result {
                        Ok(event) => {
                            process_filesystem_event(event, &index_clone);
                        }
                        Err(e) => {
                            eprintln!("Cloud watch error: {}", e);
                        }
                    }
                }

                println!("🛑 Stopped monitoring cloud folder: {}", folder_name);
            });

            watchers.push(watcher);
        }

        Ok(Self {
            _watchers: watchers,
        })
    }
}

/// Process a filesystem event and update index
fn process_filesystem_event(event: Event, index: &Arc<Mutex<FileIndex>>) {
    use EventKind::*;

    match event.kind {
        Create(_) => {
            // File created
            for path in event.paths {
                if let Ok(entry) = create_file_entry_from_path(&path) {
                    index.lock().unwrap().add_entry(entry);
                    println!("➕ Created: {}", path.display());
                }
            }
        }
        Remove(_) => {
            // File deleted
            for path in event.paths {
                let path_str = path.to_string_lossy().to_string();
                index.lock().unwrap().remove_by_path(&path_str);
                println!("➖ Removed: {}", path.display());
            }
        }
        Modify(_) => {
            // File modified
            for path in event.paths {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let path_str = path.to_string_lossy().to_string();
                    index.lock().unwrap().update_metadata_by_path(
                        &path_str,
                        metadata.len(),
                        metadata.modified().ok(),
                    );
                    println!("✏️ Modified: {}", path.display());
                }
            }
        }
        _ => {}
    }
}

/// Create a FileEntry from a filesystem path
fn create_file_entry_from_path(path: &PathBuf) -> Result<FileEntry> {
    let metadata = std::fs::metadata(path)?;
    let is_directory = metadata.is_dir();
    let size = if is_directory { 0 } else { metadata.len() };

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    let path_str = path.to_string_lossy().to_string();

    // Convert SystemTime to DateTime
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        });

    let created = metadata
        .created()
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        });

    let accessed = metadata
        .accessed()
        .ok()
        .and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
        });

    Ok(FileEntry::new(
        name,
        path_str,
        is_directory,
        0, // No MFT file ID for cloud files
        0, // No parent ID
        size,
        modified,
        created,
        accessed,
    ))
}
