use crate::file_entry::FileEntry;
use std::mem;

/// In-memory index of all files on the volume
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
}

impl Default for FileIndex {
    fn default() -> Self {
        Self::new()
    }
}
