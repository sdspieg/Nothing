use crate::file_entry::FileEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::mem;

/// In-memory index of all files on the volume
#[derive(Serialize, Deserialize)]
pub struct FileIndex {
    /// All file entries
    entries: Vec<FileEntry>,

    /// Number of directories
    directory_count: usize,
}

impl FileIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            directory_count: 0,
        }
    }

    /// Create a new index with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            directory_count: 0,
        }
    }

    /// Reserve capacity for additional entries
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
    }

    /// Add a file entry to the index
    pub fn add_entry(&mut self, entry: FileEntry) {
        if entry.is_directory {
            self.directory_count += 1;
        }
        self.entries.push(entry);
    }

    /// Get the total number of entries (files + directories)
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the number of files (non-directories)
    pub fn file_count(&self) -> usize {
        self.entries.len() - self.directory_count
    }

    /// Get the number of directories
    pub fn directory_count(&self) -> usize {
        self.directory_count
    }

    /// Estimate the memory usage of the index in bytes
    pub fn memory_usage(&self) -> usize {
        let vec_overhead = mem::size_of::<Vec<FileEntry>>();
        let entries_capacity = self.entries.capacity() * mem::size_of::<FileEntry>();

        // Estimate string data (paths and names)
        let string_data: usize = self.entries.iter()
            .map(|e| e.path.capacity() + e.name.capacity())
            .sum();

        vec_overhead + entries_capacity + string_data
    }

    /// Get a reference to all entries
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// Get a mutable reference to all entries
    pub fn entries_mut(&mut self) -> &mut Vec<FileEntry> {
        &mut self.entries
    }

    /// Remove file from index by MFT record number
    pub fn remove(&mut self, file_id: u64) {
        if let Some(pos) = self.entries.iter().position(|e| e.file_id == file_id) {
            let entry = self.entries.remove(pos);
            if entry.is_directory {
                self.directory_count = self.directory_count.saturating_sub(1);
            }
        }
    }

    /// Remove file from index by path
    pub fn remove_by_path(&mut self, path: &str) {
        if let Some(pos) = self.entries.iter().position(|e| e.path == path) {
            let entry = self.entries.remove(pos);
            if entry.is_directory {
                self.directory_count = self.directory_count.saturating_sub(1);
            }
        }
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

    /// Update metadata by path (for cloud monitoring)
    pub fn update_metadata_by_path(&mut self, path: &str, size: u64, modified: Option<std::time::SystemTime>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.path == path) {
            entry.size = size;
            if let Some(sys_time) = modified {
                if let Ok(duration) = sys_time.duration_since(std::time::UNIX_EPOCH) {
                    entry.modified = DateTime::from_timestamp(duration.as_secs() as i64, 0);
                }
            }
        }
    }
}

impl Default for FileIndex {
    fn default() -> Self {
        Self::new()
    }
}
