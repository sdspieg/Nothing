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
        let mut parent_map: HashMap<u64, Arc<str>> = HashMap::new();

        // Pre-allocate index capacity (estimate ~10M files)
        index.reserve(10_000_000);

        // TWO-PASS APPROACH to fix path resolution:
        // Pass 1: Collect directories and build parent_map
        // Pass 2: Process all entries with complete parent information

        println!("Pass 1: Collecting directories...");

        struct DirInfo {
            fid: u64,
            name: String,
            parent_fid: u64,
        }
        let mut directories: Vec<DirInfo> = Vec::new();
        let mut all_entries: Vec<usn_journal_rs::mft::MftEntry> = Vec::new();

        let mut count = 0u64;
        let progress_interval = 100_000u64;

        // Collect all entries
        for entry in mft.iter() {
            count += 1;
            if count % progress_interval == 0 {
                println!("Pass 1: {} entries...", count);
            }

            // Collect directories for path resolution
            if entry.is_dir() {
                let name = entry.file_name.to_string_lossy().to_string();
                if !name.starts_with('$') && name != "." && name != ".." {
                    directories.push(DirInfo {
                        fid: entry.fid,
                        name,
                        parent_fid: entry.parent_fid,
                    });
                }
            }

            all_entries.push(entry);
        }

        println!("Pass 1: Found {} directories, resolving paths...", directories.len());

        // Seed with root directory entry (FID 5 is standard MFT root)
        let root_path: Arc<str> = format!("{}:\\", self.drive_letter).into();
        parent_map.insert(5, Arc::clone(&root_path));

        // Iteratively resolve directory paths
        let mut unresolved = directories.len();
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 10;

        while unresolved > 0 && iteration < MAX_ITERATIONS {
            iteration += 1;
            let prev_size = parent_map.len();

            for dir in &directories {
                if parent_map.contains_key(&dir.fid) {
                    continue;
                }

                // Check if this is a root-level directory or has parent in map
                let path = if let Some(parent_path) = parent_map.get(&dir.parent_fid) {
                    let mut full_path = String::with_capacity(parent_path.len() + 1 + dir.name.len());
                    full_path.push_str(parent_path);
                    if !parent_path.ends_with('\\') {
                        full_path.push('\\');
                    }
                    full_path.push_str(&dir.name);
                    full_path.into()
                } else {
                    continue;
                };

                parent_map.insert(dir.fid, path);
            }

            let newly_resolved = parent_map.len() - prev_size;
            unresolved = directories.len() - parent_map.len();
            println!("Pass 1 iteration {}: resolved {} directories ({} remaining)",
                     iteration, newly_resolved, unresolved);

            if newly_resolved == 0 {
                break;
            }
        }

        println!("Pass 1 complete: {} directories mapped", parent_map.len());
        println!("Pass 2: Processing all files...");

        // Pass 2: Process all entries with complete parent_map
        count = 0;
        for entry in all_entries {
            count += 1;
            if count % progress_interval == 0 {
                println!("Pass 2: {} files...", count);
            }

            if let Err(e) = self.process_entry(&entry, index, &parent_map) {
                if count < 1000 {
                    eprintln!("Warning: Failed to process entry {}: {}", count, e);
                }
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
        parent_map: &HashMap<u64, Arc<str>>,
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

        // Build full path efficiently using complete parent_map from Pass 1
        let path = self.build_path_arc(&name, parent_id, parent_map);

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
