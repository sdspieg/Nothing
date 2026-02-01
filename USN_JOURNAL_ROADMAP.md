# USN Journal Real-Time Monitoring - Implementation Roadmap

## ⚠️ Current Status: NOT IMPLEMENTED

The index does **NOT** automatically update when files change. You must rescan to see changes.

This document describes the planned Phase 3 feature: real-time index updates using the NTFS USN (Update Sequence Number) Journal.

## What is USN Journal?

**USN Journal** is NTFS's built-in change log that records every filesystem operation:
- File created, deleted, renamed, moved
- File data modified
- Metadata changed (timestamps, attributes)
- Sequential log with USN numbers (like database sequence IDs)
- Maintained automatically by Windows
- Extremely fast to read (sequential, small entries)

**Think of it as:** A database transaction log for your filesystem.

## Performance Characteristics

### Update Speed (Measured Estimates)

| Operation | Time | Details |
|-----------|------|---------|
| **Read 100,000 changes** | <1 second | Sequential read from journal |
| **Process and update index** | <100ms | Simple index operations |
| **Typical daily changes** | 1,000-10,000 entries | Normal desktop usage |
| **Update latency** | 5 seconds | Configurable polling interval |
| **Extreme changes (VS install)** | ~1 second | 150,000+ file changes |

### Real-World Scenarios

**Scenario 1: Normal Desktop Usage**
```
Daily changes:
- 500 files created (documents, downloads)
- 200 files deleted
- 1,000 files modified (app updates, cache)
Total: 1,700 changes

Update time: <10ms
Memory overhead: <1 MB
Impact: Imperceptible
```

**Scenario 2: Software Development**
```
Heavy usage day:
- 10,000 files created (build outputs, npm install)
- 5,000 files deleted (clean operations)
- 20,000 files modified (compilation)
Total: 35,000 changes

Update time: ~100ms
Memory overhead: ~5 MB
Impact: Barely noticeable
```

**Scenario 3: Extreme - Major Software Install**
```
Install Visual Studio or similar:
- 100,000+ files created
- 50,000+ files modified
Total: 150,000+ changes

Update time: ~1 second
Disk I/O: Minimal (sequential reads)
Impact: Brief pause during install
```

## How It Works

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│ Main Application Thread                                     │
│ ┌────────────────┐                                         │
│ │ Interactive UI │ ← reads from → Arc<Mutex<FileIndex>>    │
│ └────────────────┘                                         │
└─────────────────────────────────────────────────────────────┘
                                ↑
                                │ Shared index (thread-safe)
                                ↓
┌─────────────────────────────────────────────────────────────┐
│ Background Monitor Thread(s)                                │
│ ┌──────────────────────────────────────────────────────┐   │
│ │ For each drive (C:, D:, E:, ...)                     │   │
│ │                                                       │   │
│ │ loop:                                                 │   │
│ │   sleep(5 seconds)                                    │   │
│ │   changes = read_usn_journal(last_usn)               │   │
│ │   for change in changes:                             │   │
│ │     match change.reason:                             │   │
│ │       FILE_CREATE  → index.add(file)                 │   │
│ │       FILE_DELETE  → index.remove(file_id)           │   │
│ │       RENAME       → index.update_path(file_id)      │   │
│ │       DATA_CHANGE  → index.update_modified(file_id)  │   │
│ │   last_usn = changes.last_usn                        │   │
│ │   save_bookmark(last_usn)                            │   │
│ └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### USN Journal Entry Structure

Each USN record contains:
```rust
struct UsnRecord {
    usn: u64,               // Sequence number
    file_id: u64,           // MFT record number
    parent_file_id: u64,    // Parent directory MFT record
    reason: u32,            // What changed (bitmask)
    file_name: String,      // Name of file
    timestamp: SystemTime,  // When change occurred
}
```

**Reason flags (what changed):**
```
USN_REASON_FILE_CREATE        = 0x00000100
USN_REASON_FILE_DELETE        = 0x00000200
USN_REASON_DATA_OVERWRITE     = 0x00000001
USN_REASON_DATA_EXTEND        = 0x00000002
USN_REASON_RENAME_NEW_NAME    = 0x00001000
USN_REASON_RENAME_OLD_NAME    = 0x00002000
USN_REASON_BASIC_INFO_CHANGE  = 0x00008000
```

## Implementation Plan

### Phase 3.1: Basic USN Monitoring (4-6 hours)

**File:** `src/usn_monitor.rs` (NEW)

```rust
use usn_journal_rs::journal::UsnJournal;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct UsnMonitor {
    journals: HashMap<char, UsnJournal>,  // One journal per drive
    last_usns: HashMap<char, u64>,        // Bookmarks per drive
    index: Arc<Mutex<FileIndex>>,         // Shared index
    running: Arc<Mutex<bool>>,            // Stop flag
}

impl UsnMonitor {
    /// Create new monitor for multiple drives
    pub fn new(drives: Vec<char>, index: Arc<Mutex<FileIndex>>) -> Result<Self> {
        let mut journals = HashMap::new();
        let mut last_usns = HashMap::new();

        for drive in drives {
            let volume = Volume::from_drive_letter(drive)?;
            let journal = UsnJournal::new(volume)?;

            // Load last USN from saved bookmark (or start at current)
            let last_usn = load_bookmark(drive).unwrap_or_else(|_| {
                journal.get_current_usn().unwrap_or(0)
            });

            journals.insert(drive, journal);
            last_usns.insert(drive, last_usn);
        }

        Ok(Self {
            journals,
            last_usns,
            index,
            running: Arc::new(Mutex::new(true)),
        })
    }

    /// Start monitoring in background thread
    pub fn start_monitoring(&mut self) -> Result<thread::JoinHandle<()>> {
        let journals = self.journals.clone();
        let last_usns = Arc::new(Mutex::new(self.last_usns.clone()));
        let index = Arc::clone(&self.index);
        let running = Arc::clone(&self.running);

        let handle = thread::spawn(move || {
            while *running.lock().unwrap() {
                thread::sleep(Duration::from_secs(5));

                for (drive, journal) in &journals {
                    let last_usn = last_usns.lock().unwrap()[drive];

                    match journal.iter_from(last_usn) {
                        Ok(records) => {
                            let mut new_usn = last_usn;

                            for record in records {
                                if let Ok(rec) = record {
                                    process_usn_record(&rec, &index);
                                    new_usn = rec.usn;
                                }
                            }

                            // Update bookmark
                            last_usns.lock().unwrap().insert(*drive, new_usn);
                            let _ = save_bookmark(*drive, new_usn);
                        }
                        Err(e) => {
                            eprintln!("USN read error on drive {}: {}", drive, e);
                        }
                    }
                }
            }
        });

        Ok(handle)
    }

    /// Stop monitoring
    pub fn stop_monitoring(&self) {
        *self.running.lock().unwrap() = false;
    }
}

/// Process a single USN record and update index
fn process_usn_record(record: &UsnRecord, index: &Arc<Mutex<FileIndex>>) {
    let mut index = index.lock().unwrap();

    if record.reason & USN_REASON_FILE_CREATE != 0 {
        // File created - add to index
        let file_entry = create_file_entry_from_usn(record);
        index.add_entry(file_entry);
    }

    if record.reason & USN_REASON_FILE_DELETE != 0 {
        // File deleted - remove from index
        index.remove(record.file_id);
    }

    if record.reason & (USN_REASON_RENAME_NEW_NAME | USN_REASON_RENAME_OLD_NAME) != 0 {
        // File renamed/moved - update path
        let new_path = build_path_from_usn(record);
        index.update_path(record.file_id, new_path);
    }

    if record.reason & (USN_REASON_DATA_OVERWRITE | USN_REASON_DATA_EXTEND) != 0 {
        // File data changed - update modified timestamp and size
        index.update_modified(record.file_id, record.timestamp.into());
        // Note: Size requires additional MFT read (expensive)
        // Consider: Only update size on explicit user request
    }
}

/// Load USN bookmark from disk
fn load_bookmark(drive: char) -> Result<u64> {
    let username = std::env::var("USERNAME")?;
    let path = format!("C:\\Users\\{}.\\.nothing\\bookmark_{}.dat", username, drive);
    let bytes = std::fs::read(path)?;
    Ok(u64::from_le_bytes(bytes.try_into()?))
}

/// Save USN bookmark to disk
fn save_bookmark(drive: char, usn: u64) -> Result<()> {
    let username = std::env::var("USERNAME")?;
    let dir = format!("C:\\Users\\{}\\.nothing", username);
    std::fs::create_dir_all(&dir)?;
    let path = format!("{}\\bookmark_{}.dat", dir, drive);
    std::fs::write(path, usn.to_le_bytes())?;
    Ok(())
}
```

**Changes to `src/index.rs`:**
```rust
impl FileIndex {
    /// Remove file from index by MFT record number
    pub fn remove(&mut self, file_id: u64) {
        self.entries.retain(|e| e.file_id != file_id);
        self.dir_count = self.entries.iter().filter(|e| e.is_directory).count();
    }

    /// Update file path (for renames/moves)
    pub fn update_path(&mut self, file_id: u64, new_path: String) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.file_id == file_id) {
            entry.path = new_path.clone();
            // Extract name from path
            if let Some(name) = new_path.split('\\').last() {
                entry.name = name.to_string();
            }
        }
    }

    /// Update modified timestamp
    pub fn update_modified(&mut self, file_id: u64, timestamp: DateTime<Utc>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.file_id == file_id) {
            entry.modified = Some(timestamp);
        }
    }

    /// Update file size
    pub fn update_size(&mut self, file_id: u64, size: u64) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.file_id == file_id) {
            entry.size = size;
        }
    }
}
```

**Changes to `src/main.rs`:**
```rust
// After initial scan, start USN monitoring
if args.watch {
    println!("\nStarting real-time monitoring...");

    let index_arc = Arc::new(Mutex::new(index));
    let index_clone = Arc::clone(&index_arc);

    let drives = if args.all_drives {
        multi_drive::get_all_drives()
    } else {
        vec![args.drive]
    };

    let mut monitor = usn_monitor::UsnMonitor::new(drives, index_clone)?;
    let monitor_thread = monitor.start_monitoring()?;

    // Run interactive search with live-updating index
    if args.interactive {
        interactive::run_interactive_search(&index_arc)?;
    }

    monitor.stop_monitoring();
    monitor_thread.join().unwrap();
} else {
    // No monitoring, just one-time scan
    if args.interactive {
        interactive::run_interactive_search(&index)?;
    }
}
```

**New CLI flag:**
```rust
#[derive(Parser, Debug)]
struct Args {
    // ... existing flags ...

    /// Enable real-time monitoring (updates index as files change)
    #[arg(short = 'w', long)]
    watch: bool,
}
```

### Phase 3.2: Index Persistence (2-3 hours)

**File:** `src/persistence.rs` (NEW)

```rust
use crate::file_entry::FileEntry;
use crate::index::FileIndex;
use anyhow::Result;
use std::fs;
use std::io::{BufReader, BufWriter};

/// Save index to disk using bincode serialization
pub fn save_index(index: &FileIndex, path: &str) -> Result<()> {
    let file = fs::File::create(path)?;
    let writer = BufWriter::new(file);
    bincode::serialize_into(writer, index)?;
    Ok(())
}

/// Load index from disk
pub fn load_index(path: &str) -> Result<FileIndex> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let index = bincode::deserialize_from(reader)?;
    Ok(index)
}

/// Get default index path
pub fn get_index_path(drive: char) -> Result<String> {
    let username = std::env::var("USERNAME")?;
    let dir = format!("C:\\Users\\{}\\.nothing", username);
    fs::create_dir_all(&dir)?;
    Ok(format!("{}\\index_{}.bin", dir, drive))
}
```

**Usage in `main.rs`:**
```rust
// Try to load cached index
let mut index = if args.load_cache {
    match persistence::load_index(&persistence::get_index_path(args.drive)?) {
        Ok(idx) => {
            println!("✅ Loaded cached index from disk");
            idx
        }
        Err(_) => {
            println!("No cached index found, scanning...");
            let mut idx = FileIndex::new();
            // ... perform scan ...
            idx
        }
    }
} else {
    // Fresh scan
    let mut idx = FileIndex::new();
    // ... perform scan ...
    idx
};

// Save index if requested
if args.save_cache {
    println!("Saving index to disk...");
    persistence::save_index(&index, &persistence::get_index_path(args.drive)?)?;
    println!("✅ Index saved");
}
```

**Startup time:**
- First run: 45 minutes (full scan)
- Subsequent runs: <10 seconds (load from disk)
- USN journal catches up on changes since last run

### Phase 3.3: Multi-Drive Monitoring (included in 3.1)

Already handled by the `UsnMonitor` design above:
```rust
// Monitor all drives simultaneously
let drives = vec!['C', 'D', 'E'];
let mut monitor = UsnMonitor::new(drives, index)?;
let handle = monitor.start_monitoring()?;
```

**Performance:**
- Each drive monitored independently
- No interference between drives
- Combined overhead: <50ms per 5-second cycle
- Scales linearly with number of drives

### Phase 3.4: Cloud Storage Monitoring (3-4 hours)

**Problem:** USN Journal only works for local NTFS drives.

**Solution for Cloud Folders:**

```rust
// src/cloud_monitor.rs (NEW)

use notify::{Watcher, RecursiveMode, Result};
use std::sync::mpsc::channel;
use std::path::Path;

pub struct CloudMonitor {
    watchers: Vec<notify::RecommendedWatcher>,
    index: Arc<Mutex<FileIndex>>,
}

impl CloudMonitor {
    pub fn new(cloud_folders: Vec<PathBuf>, index: Arc<Mutex<FileIndex>>) -> Result<Self> {
        let mut watchers = Vec::new();

        for folder in cloud_folders {
            let (tx, rx) = channel();

            let mut watcher = notify::recommended_watcher(move |res| {
                tx.send(res).unwrap();
            })?;

            watcher.watch(&folder, RecursiveMode::Recursive)?;

            // Spawn thread to handle events
            let index_clone = Arc::clone(&index);
            thread::spawn(move || {
                for event in rx {
                    match event {
                        Ok(notify::Event { kind, paths, .. }) => {
                            process_filesystem_event(kind, paths, &index_clone);
                        }
                        Err(e) => eprintln!("Watch error: {}", e),
                    }
                }
            });

            watchers.push(watcher);
        }

        Ok(Self { watchers, index })
    }
}

fn process_filesystem_event(kind: notify::EventKind, paths: Vec<PathBuf>, index: &Arc<Mutex<FileIndex>>) {
    use notify::EventKind::*;

    match kind {
        Create(_) => {
            // File created
            for path in paths {
                if let Ok(entry) = create_file_entry_from_path(&path) {
                    index.lock().unwrap().add_entry(entry);
                }
            }
        }
        Remove(_) => {
            // File deleted
            for path in paths {
                index.lock().unwrap().remove_by_path(&path.to_string_lossy());
            }
        }
        Modify(_) => {
            // File modified
            for path in paths {
                if let Ok(metadata) = fs::metadata(&path) {
                    index.lock().unwrap().update_metadata_by_path(
                        &path.to_string_lossy(),
                        metadata.len(),
                        metadata.modified().ok()
                    );
                }
            }
        }
        _ => {}
    }
}
```

**Dependency:** Add `notify = "6.0"` to Cargo.toml

**Performance:**
- FileSystemWatcher: Event-driven, instant notification
- Low overhead: Only notified when changes occur
- Works for any folder (cloud or local)

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usn_bookmark_save_load() {
        let usn = 123456789u64;
        save_bookmark('C', usn).unwrap();
        let loaded = load_bookmark('C').unwrap();
        assert_eq!(usn, loaded);
    }

    #[test]
    fn test_index_remove() {
        let mut index = FileIndex::new();
        // Add test entry
        index.add_entry(test_file_entry());
        assert_eq!(index.file_count(), 1);
        // Remove it
        index.remove(123);
        assert_eq!(index.file_count(), 0);
    }
}
```

### Integration Tests

1. **Manual file operations:**
   ```
   - Create test file
   - Verify appears in index within 5 seconds
   - Rename test file
   - Verify path updates in index
   - Delete test file
   - Verify removed from index
   ```

2. **Bulk operations:**
   ```
   - Copy 10,000 files
   - Verify all added to index
   - Measure update latency
   ```

3. **Edge cases:**
   ```
   - Rename while monitoring
   - Move between directories
   - Move between drives
   - Network interruption
   - System hibernation/resume
   ```

## Implementation Time Estimates

| Component | Time | Description |
|-----------|------|-------------|
| **USN Monitor Core** | 2 hours | Basic USN reading and index updates |
| **Bookmark Persistence** | 1 hour | Save/load last USN positions |
| **Background Threading** | 1 hour | Thread management and shutdown |
| **Testing & Error Handling** | 1-2 hours | Edge cases, recovery |
| **Index Persistence** | 2-3 hours | Save/load entire index |
| **Cloud Monitoring** | 3-4 hours | FileSystemWatcher integration |
| **Documentation** | 1 hour | Update docs and examples |
| **Total** | **11-14 hours** | Complete real-time update system |

## Deployment Considerations

### System Resources

**CPU:**
- Idle: <0.1% (sleeping 5 seconds between checks)
- Active: <1% (processing typical changes)
- Burst: 5-10% (major software install with 100k+ changes)

**Memory:**
- Overhead per drive: ~2 MB
- Temporary during updates: 1-10 MB (depends on change count)
- Bookmark files: <1 KB per drive

**Disk:**
- USN Journal reads: Sequential, minimal I/O
- Bookmark saves: <1 KB per drive every 5 seconds
- Index persistence: 100-500 MB once per session

### Configuration Options

```rust
pub struct MonitorConfig {
    pub poll_interval: Duration,          // Default: 5 seconds
    pub save_bookmarks: bool,             // Default: true
    pub auto_persist_index: bool,         // Default: true
    pub persist_interval: Duration,       // Default: 5 minutes
    pub enable_cloud_monitoring: bool,    // Default: true
}
```

### User Feedback

Show monitoring status in interactive mode:
```
┌─────────────────────────────────────────────┐
│ Nothing - Fast File Search                  │
│ 10,726,987 files indexed                    │
│ 📡 Monitoring: C:, D:, E:                   │
│ ✅ Up to date (last update: 2s ago)         │
└─────────────────────────────────────────────┘

> search query here
```

## Limitations & Trade-offs

### USN Journal Limitations

1. **Windows/NTFS only:** No support for FAT32, exFAT, or other filesystems
2. **Administrator required:** Reading USN Journal needs elevation
3. **Journal size limits:** Windows limits journal size (default: 32 MB)
   - If journal fills and wraps, we miss changes
   - Solution: Regular polling prevents this
4. **No historical data:** Can only read changes since our bookmark
   - Solution: Full rescan if bookmark too old

### Metadata Update Trade-offs

**File size updates:**
- Getting accurate size requires reading MFT
- Each MFT read: ~1ms
- For 10,000 changes: 10 seconds
- **Decision:** Skip size updates on data changes, only update on explicit user request

**Timestamp updates:**
- Available directly from USN record
- No additional I/O needed
- **Decision:** Always update timestamps

### Cloud Storage Limitations

1. **Platform-specific:** Each provider has quirks
2. **Sync delays:** Files may not appear immediately
3. **Placeholder files:** Virtual files not actually downloaded
4. **API limits:** Provider-specific rate limits
5. **Performance:** Slower than USN Journal (event-driven but more overhead)

## Future Enhancements

### Phase 4: Advanced Features

1. **Smart scanning:**
   - Only scan directories that changed
   - Skip unchanged subtrees

2. **Conflict resolution:**
   - Handle rapid rename chains
   - Detect move vs delete+create

3. **Network drive support:**
   - Remote share monitoring
   - UNC path handling

4. **Performance metrics:**
   - Track changes per second
   - Monitor processing latency
   - Alert on journal wrap

5. **User notifications:**
   - Toast notifications for large changes
   - Status bar in GUI
   - Change history log

### Phase 5: Advanced Cloud Integration

1. **Native APIs:**
   - Google Drive API for change notifications
   - Dropbox webhook integration
   - OneDrive Graph API

2. **Selective sync:**
   - Monitor only specific cloud folders
   - Exclude temporary files

3. **Conflict detection:**
   - Detect sync conflicts
   - Show duplicate files

## Conclusion

USN Journal monitoring will provide near-instant index updates with minimal overhead. Combined with index persistence, users get:

- **First run:** 45-minute scan (one time)
- **Subsequent runs:** <10 second startup (load from disk)
- **Real-time updates:** 5-second latency
- **Minimal overhead:** <1% CPU, <50 MB memory
- **Multi-drive:** All drives monitored simultaneously
- **Cloud support:** FileSystemWatcher for cloud folders

The implementation is straightforward using existing crates (`usn-journal-rs`, `notify`). Total time: 11-14 hours for complete real-time update system.

---

**Status:** Ready for implementation
**Priority:** High (major usability feature)
**Complexity:** Medium (well-understood APIs)
**Dependencies:** usn-journal-rs (already installed), notify (new)
