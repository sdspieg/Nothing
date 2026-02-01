use crate::index::FileIndex;
use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// Monitors filesystem for changes using notify crate
/// This is a simpler approach that works for all filesystems
pub struct UsnMonitor {
    _watchers: Vec<notify::RecommendedWatcher>,
}

impl UsnMonitor {
    /// Create new monitor for multiple drives
    pub fn new(drives: Vec<char>, index: Arc<Mutex<FileIndex>>) -> Result<Self> {
        let mut watchers = Vec::new();

        for drive in drives {
            let drive_path = PathBuf::from(format!("{}:\\", drive));

            if !drive_path.exists() {
                eprintln!("Drive {}:\\ not found, skipping monitoring", drive);
                continue;
            }

            let index_clone = Arc::clone(&index);
            let (tx, rx) = std::sync::mpsc::channel();

            let mut watcher = notify::recommended_watcher(move |res| {
                if let Err(e) = tx.send(res) {
                    eprintln!("Monitor send error: {}", e);
                }
            })?;

            // Watch the drive root
            match watcher.watch(&drive_path, RecursiveMode::Recursive) {
                Ok(_) => {
                    println!("📡 Monitoring drive {}:\\", drive);

                    // Spawn thread to handle events
                    thread::spawn(move || {
                        for event_result in rx {
                            match event_result {
                                Ok(event) => {
                                    process_filesystem_event(event, &index_clone);
                                }
                                Err(e) => {
                                    eprintln!("Watch error on drive {}: {}", drive, e);
                                }
                            }
                        }
                        println!("🛑 Stopped monitoring drive {}:\\", drive);
                    });

                    watchers.push(watcher);
                }
                Err(e) => {
                    eprintln!("Failed to watch drive {}: {}", drive, e);
                }
            }
        }

        if watchers.is_empty() {
            eprintln!("⚠️  No drives could be monitored");
        }

        Ok(Self {
            _watchers: watchers,
        })
    }

    /// Stop monitoring (handled automatically when dropped)
    pub fn stop(self) {
        // Watchers will be dropped and stop automatically
        println!("Stopping filesystem monitoring...");
    }
}

/// Process a filesystem event and update index
fn process_filesystem_event(event: Event, index: &Arc<Mutex<FileIndex>>) {
    use EventKind::*;

    match event.kind {
        Create(_) => {
            for path in event.paths {
                if let Ok(entry) = create_file_entry_from_path(&path) {
                    index.lock().unwrap().add_entry(entry);
                    println!("➕ Created: {}", path.display());
                }
            }
        }
        Remove(_) => {
            for path in event.paths {
                let path_str = path.to_string_lossy().to_string();
                index.lock().unwrap().remove_by_path(&path_str);
                println!("➖ Removed: {}", path.display());
            }
        }
        Modify(_) => {
            for path in event.paths {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let path_str = path.to_string_lossy().to_string();
                    index.lock().unwrap().update_metadata_by_path(
                        &path_str,
                        metadata.len(),
                        metadata.modified().ok(),
                    );
                    println!("✏️  Modified: {}", path.display());
                }
            }
        }
        _ => {}
    }
}

/// Create a FileEntry from a filesystem path
fn create_file_entry_from_path(path: &PathBuf) -> Result<crate::file_entry::FileEntry> {
    use crate::file_entry::FileEntry;

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
        0, // No MFT file ID
        0, // No parent ID
        size,
        modified,
        created,
        accessed,
    ))
}
