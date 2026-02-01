use crate::file_entry::FileEntry;
use crate::index::FileIndex;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use usn_journal_rs::mft::Mft;
use usn_journal_rs::volume::Volume;

/// MFT reader for scanning NTFS volumes
pub struct MftReader {
    drive_letter: char,
}

impl MftReader {
    /// Create a new MFT reader for the specified drive
    pub fn new(drive_letter: char) -> Result<Self> {
        // Validate drive letter
        if !drive_letter.is_ascii_alphabetic() {
            anyhow::bail!("Invalid drive letter: {}", drive_letter);
        }

        Ok(Self {
            drive_letter: drive_letter.to_ascii_uppercase(),
        })
    }

    /// Scan the MFT and populate the file index
    pub fn scan_into_index(&self, index: &mut FileIndex) -> Result<()> {
        println!("Nothing - Fast File Search Tool");
        println!("Scanning drive {}:...\n", self.drive_letter);

        let start_time = Instant::now();

        // Open the volume
        let volume = Volume::from_drive_letter(self.drive_letter)
            .with_context(|| {
                format!(
                    "Failed to open volume {}:\nMake sure you're running as Administrator",
                    self.drive_letter
                )
            })?;

        // Create MFT reader
        let mft = Mft::new(volume);

        // Track parent directories to build full paths
        // Use Arc<str> to avoid cloning large strings
        let mut parent_map: HashMap<u64, Arc<str>> = HashMap::new();

        // Pre-allocate index capacity (estimate ~10M files)
        index.reserve(10_000_000);

        // Iterate through MFT entries
        let mut count = 0u64;
        let progress_interval = 100_000u64;

        for entry in mft.iter() {
            count += 1;

            // Show progress every 100k files (optimized check)
            if count % progress_interval == 0 {
                println!("Progress: {} files...", count);
            }

            // Process the entry
            if let Err(e) = self.process_entry(&entry, index, &mut parent_map) {
                // Log but don't fail on individual entry errors
                if count < 1000 {
                    // Only log first 1000 errors to avoid spam
                    eprintln!("Warning: Failed to process entry {}: {}", count, e);
                }
            }

            // Periodically clean up old parent_map entries to save memory
            // Keep only recent parents (entries processed in last 1M files)
            if count % 1_000_000 == 0 && count > 1_000_000 {
                self.cleanup_parent_map(&mut parent_map, &entry);
            }
        }

        let elapsed = start_time.elapsed();
        let files_per_sec = if elapsed.as_secs() > 0 {
            count / elapsed.as_secs()
        } else {
            count
        };

        println!("\nScan complete!");
        println!("Total files: {}", index.file_count());
        println!("Directories: {}", index.directory_count());
        println!(
            "Time taken: {:.2} seconds ({} files/sec)",
            elapsed.as_secs_f64(),
            files_per_sec
        );
        println!("Index memory: {}", format_size(index.memory_usage() as u64));

        Ok(())
    }

    /// Process a single MFT entry (optimized)
    #[inline]
    fn process_entry(
        &self,
        entry: &usn_journal_rs::mft::MftEntry,
        index: &mut FileIndex,
        parent_map: &mut HashMap<u64, Arc<str>>,
    ) -> Result<()> {
        // Get file name (convert OsString to String)
        let name = entry.file_name.to_string_lossy();

        // Skip system files and . / .. entries (early return)
        if name.starts_with('$') || name.len() <= 2 && (name == "." || name == "..") {
            return Ok(());
        }

        // Get file attributes
        let file_id = entry.fid;
        let parent_id = entry.parent_fid;
        let is_directory = entry.is_dir();

        // Build full path efficiently
        let path = self.build_path_arc(&name, parent_id, parent_map);

        // Store this entry's path for its children (using Arc, no clone!)
        if is_directory {
            parent_map.insert(file_id, Arc::clone(&path));
        }

        // Create file entry
        // Convert Arc<str> to String (cheap if Arc has single reference)
        // Fast mode doesn't have size/timestamp info
        let file_entry = FileEntry::new(
            name.into_owned(),
            path.to_string(),
            is_directory,
            file_id,
            parent_id,
            0,    // size not available in fast mode
            None, // modified not available
            None, // created not available
            None, // accessed not available
        );

        index.add_entry(file_entry);

        Ok(())
    }

    /// Build the full path for a file (optimized with Arc)
    #[inline]
    fn build_path_arc(
        &self,
        name: &str,
        parent_id: u64,
        parent_map: &HashMap<u64, Arc<str>>,
    ) -> Arc<str> {
        // Root directory (MFT entry 5)
        if parent_id == 5 || parent_id == 0 {
            return format!("{}:\\{}", self.drive_letter, name).into();
        }

        // Try to find parent path
        if let Some(parent_path) = parent_map.get(&parent_id) {
            // Use String::with_capacity to pre-allocate
            let mut full_path = String::with_capacity(parent_path.len() + 1 + name.len());
            full_path.push_str(parent_path);
            full_path.push('\\');
            full_path.push_str(name);
            full_path.into()
        } else {
            // Parent not found, use drive root
            format!("{}:\\{}", self.drive_letter, name).into()
        }
    }

    /// Clean up old parent_map entries to reduce memory usage
    fn cleanup_parent_map(
        &self,
        parent_map: &mut HashMap<u64, Arc<str>>,
        _current_entry: &usn_journal_rs::mft::MftEntry,
    ) {
        // Only keep parents that are still referenced (Arc::strong_count > 1)
        parent_map.retain(|_, path| Arc::strong_count(path) > 1);
    }
}

/// Format a size in bytes to human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[0])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}
