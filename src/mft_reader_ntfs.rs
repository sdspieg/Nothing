use crate::file_entry::FileEntry;
use crate::index::FileIndex;
use crate::sector_aligned_reader::SectorAlignedReader;
use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use ntfs::structured_values::{NtfsFileName, NtfsFileNamespace};
use ntfs::{Ntfs, NtfsFile};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::sync::Arc;
use std::time::Instant;

/// MFT reader using ntfs crate for full metadata
pub struct MftReaderNtfs {
    drive_letter: char,
}

impl MftReaderNtfs {
    /// Create a new MFT reader for the specified drive
    pub fn new(drive_letter: char) -> Result<Self> {
        if !drive_letter.is_ascii_alphabetic() {
            anyhow::bail!("Invalid drive letter: {}", drive_letter);
        }

        Ok(Self {
            drive_letter: drive_letter.to_ascii_uppercase(),
        })
    }

    /// Scan the MFT and populate the file index
    pub fn scan_into_index(&self, index: &mut FileIndex) -> Result<()> {
        println!("Nothing - Fast File Search Tool (Full Metadata)");
        println!("Scanning drive {}:...\n", self.drive_letter);

        let start_time = Instant::now();

        // Open volume with sector-aligned reader
        let volume_path = format!("\\\\.\\{}:", self.drive_letter);
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&volume_path)
            .with_context(|| {
                format!(
                    "Failed to open volume {}:\nMake sure you're running as Administrator",
                    self.drive_letter
                )
            })?;

        let mut file = SectorAlignedReader::new(file);

        // Initialize NTFS
        let mut ntfs = Ntfs::new(&mut file)?;
        ntfs.read_upcase_table(&mut file)?;

        // Get root directory (verify it exists)
        let _root_dir = ntfs.root_directory(&mut file)?;

        // Track parent directories for path building
        let mut parent_map: HashMap<u64, Arc<str>> = HashMap::new();

        // Pre-allocate index
        index.reserve(10_000_000);

        // TWO-PASS APPROACH to fix path resolution issues:
        // Pass 1: Build complete parent_map with all directory paths
        // Pass 2: Process all files with complete parent information

        println!("Pass 1: Collecting directories...");

        // Store directory info for processing
        struct DirInfo {
            record_number: u64,
            name: String,
            parent_id: u64,
        }
        let mut directories: Vec<DirInfo> = Vec::new();

        let mut pass1_count = 0u64;
        let mut consecutive_errors = 0;
        let max_consecutive_errors = 1000;
        let progress_interval = 100_000u64;

        // Collect all directories first
        for record_number in 0..u64::MAX {
            match ntfs.file(&mut file, record_number) {
                Ok(ntfs_file) => {
                    consecutive_errors = 0;
                    pass1_count += 1;

                    if pass1_count % progress_interval == 0 {
                        println!("Pass 1: {} entries...", pass1_count);
                    }

                    // Only collect directories in pass 1
                    if ntfs_file.is_directory() {
                        if let Ok(file_name) = self.get_best_file_name(&mut file, &ntfs_file) {
                            let name = file_name.name().to_string_lossy().to_string();
                            if !name.starts_with('$') && name != "." && name != ".." {
                                let parent_id = file_name.parent_directory_reference().file_record_number();
                                directories.push(DirInfo {
                                    record_number,
                                    name,
                                    parent_id,
                                });
                            }
                        }
                    }
                }
                Err(_) => {
                    consecutive_errors += 1;
                    if consecutive_errors >= max_consecutive_errors {
                        break;
                    }
                }
            }
        }

        println!("Pass 1: Found {} directories, resolving paths...", directories.len());

        // Seed with root directory entry (MFT entry 5 is standard NTFS root)
        let root_path: Arc<str> = format!("{}:\\", self.drive_letter).into();
        parent_map.insert(5, Arc::clone(&root_path));

        // Iteratively resolve directory paths until stable
        let mut unresolved = directories.len();
        let mut iteration = 0;
        const MAX_ITERATIONS: usize = 10;

        while unresolved > 0 && iteration < MAX_ITERATIONS {
            iteration += 1;
            let prev_size = parent_map.len();

            for dir in &directories {
                let file_id = dir.record_number;

                // Skip if already resolved
                if parent_map.contains_key(&file_id) {
                    continue;
                }

                // Try to build path using parent from map
                let path = if let Some(parent_path) = parent_map.get(&dir.parent_id) {
                    // Parent found - build full path
                    let mut full_path = String::with_capacity(parent_path.len() + 1 + dir.name.len());
                    full_path.push_str(parent_path);
                    if !parent_path.ends_with('\\') {
                        full_path.push('\\');
                    }
                    full_path.push_str(&dir.name);
                    full_path.into()
                } else {
                    // Parent not found yet - skip this iteration
                    continue;
                };

                parent_map.insert(file_id, path);
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

        // Reset for pass 2
        println!("Pass 2: Processing all files...");
        let mut pass2_count = 0u64;
        consecutive_errors = 0;

        for record_number in 0..u64::MAX {
            match ntfs.file(&mut file, record_number) {
                Ok(ntfs_file) => {
                    consecutive_errors = 0;
                    pass2_count += 1;

                    if pass2_count % progress_interval == 0 {
                        println!("Pass 2: {} files...", pass2_count);
                    }

                    // Process all files with complete parent_map
                    if let Err(e) = self.process_ntfs_file(
                        &mut file,
                        &ntfs,
                        &ntfs_file,
                        index,
                        &parent_map,
                    ) {
                        if pass2_count < 1000 {
                            eprintln!("Warning: Failed to process record {}: {}", record_number, e);
                        }
                    }
                }
                Err(_) => {
                    consecutive_errors += 1;
                    if consecutive_errors >= max_consecutive_errors {
                        break;
                    }
                }
            }
        }

        let elapsed = start_time.elapsed();
        let files_per_sec = if elapsed.as_secs() > 0 {
            pass2_count / elapsed.as_secs()
        } else {
            pass2_count
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

    /// Build directory path and add to parent_map (Pass 1)
    fn build_directory_path<'a, T>(
        &self,
        fs: &mut T,
        ntfs_file: &NtfsFile<'a>,
        parent_map: &mut HashMap<u64, Arc<str>>,
    ) -> Result<()>
    where
        T: std::io::Read + std::io::Seek,
    {
        let file_name = self.get_best_file_name(fs, ntfs_file)?;
        let name = file_name.name().to_string_lossy().to_string();

        // Skip system files
        if name.starts_with('$') || name == "." || name == ".." {
            return Ok(());
        }

        let file_id = ntfs_file.file_record_number();
        let parent_id = file_name.parent_directory_reference().file_record_number();

        // Build path recursively following parent chain
        let path = self.build_path_recursive(&name, parent_id, parent_map);
        parent_map.insert(file_id, path);

        Ok(())
    }

    /// Build path recursively by following parent chain
    fn build_path_recursive(
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
            let mut full_path = String::with_capacity(parent_path.len() + 1 + name.len());
            full_path.push_str(parent_path);
            full_path.push('\\');
            full_path.push_str(name);
            full_path.into()
        } else {
            // Parent not found - this can happen for nested directories
            // Default to root and let subsequent iterations fix it
            format!("{}:\\{}", self.drive_letter, name).into()
        }
    }

    /// Process a single NTFS file entry (Pass 2)
    fn process_ntfs_file<'a, T>(
        &self,
        fs: &mut T,
        _ntfs: &Ntfs,
        ntfs_file: &NtfsFile<'a>,
        index: &mut FileIndex,
        parent_map: &HashMap<u64, Arc<str>>,
    ) -> Result<()>
    where
        T: std::io::Read + std::io::Seek,
    {
        // Get file name (prefer Win32 namespace, fallback to others)
        let file_name = self.get_best_file_name(fs, ntfs_file)?;
        let name = file_name.name().to_string_lossy().to_string();

        // Skip system files and special entries
        if name.starts_with('$') || name == "." || name == ".." {
            return Ok(());
        }

        let file_id = ntfs_file.file_record_number();
        let parent_id = file_name.parent_directory_reference().file_record_number();
        let is_directory = ntfs_file.is_directory();

        // Get file size (0 for directories)
        let size = if is_directory {
            0
        } else {
            self.get_file_size(fs, ntfs_file).unwrap_or(0)
        };

        // Get timestamps
        let (created, modified, accessed) = self.get_timestamps(fs, ntfs_file)?;

        // Build full path using complete parent_map from Pass 1
        let path = self.build_path_arc(&name, parent_id, parent_map);

        // Create file entry
        let file_entry = FileEntry::new(
            name,
            path.to_string(),
            is_directory,
            file_id,
            parent_id,
            size,
            modified,
            created,
            accessed,
        );

        index.add_entry(file_entry);

        Ok(())
    }

    /// Get the best file name (prefer Win32 namespace)
    fn get_best_file_name<'a, T>(&self, fs: &mut T, ntfs_file: &'a NtfsFile<'a>) -> Result<NtfsFileName>
    where
        T: std::io::Read + std::io::Seek,
    {
        use ntfs::NtfsAttributeType;

        // Try to find Win32 or Win32+DOS namespace first
        let mut fallback = None;
        let mut iter = ntfs_file.attributes();

        while let Some(attr_item_result) = iter.next(fs) {
            let attr_item = attr_item_result?;
            let attr = attr_item.to_attribute()?;
            if attr.ty()? == NtfsAttributeType::FileName {
                let name_attr: NtfsFileName = attr.structured_value(fs)?;
                let namespace = name_attr.namespace();
                match namespace {
                    NtfsFileNamespace::Win32 | NtfsFileNamespace::Win32AndDos => {
                        return Ok(name_attr);
                    }
                    _ => {
                        if fallback.is_none() {
                            fallback = Some(name_attr);
                        }
                    }
                }
            }
        }

        fallback.ok_or_else(|| anyhow::anyhow!("No file name attribute found"))
    }

    /// Get file size from data attribute
    fn get_file_size<T>(&self, fs: &mut T, ntfs_file: &NtfsFile) -> Result<u64>
    where
        T: std::io::Read + std::io::Seek,
    {
        use ntfs::NtfsAttributeType;

        // Look for $DATA attribute
        let mut iter = ntfs_file.attributes();
        while let Some(attr_item_result) = iter.next(fs) {
            let attr_item = attr_item_result?;
            let attr = attr_item.to_attribute()?;
            if attr.ty()? == NtfsAttributeType::Data {
                // $DATA attribute type
                return Ok(attr.value_length());
            }
        }
        Ok(0)
    }

    /// Get timestamps from standard information attribute
    fn get_timestamps<T>(
        &self,
        fs: &mut T,
        ntfs_file: &NtfsFile,
    ) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<DateTime<Utc>>)>
    where
        T: std::io::Read + std::io::Seek,
    {
        use ntfs::structured_values::NtfsStandardInformation;
        use ntfs::NtfsAttributeType;

        // Look for $STANDARD_INFORMATION attribute
        let mut iter = ntfs_file.attributes();
        while let Some(attr_item_result) = iter.next(fs) {
            let attr_item = attr_item_result?;
            let attr = attr_item.to_attribute()?;
            if attr.ty()? == NtfsAttributeType::StandardInformation {
                let std_info: NtfsStandardInformation = attr.structured_value(fs)?;
                let created = filetime_to_datetime(std_info.creation_time().nt_timestamp());
                let modified = filetime_to_datetime(std_info.modification_time().nt_timestamp());
                let accessed = filetime_to_datetime(std_info.access_time().nt_timestamp());
                return Ok((created, modified, accessed));
            }
        }

        Ok((None, None, None))
    }

    /// Build full path using Arc for efficiency
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
            let mut full_path = String::with_capacity(parent_path.len() + 1 + name.len());
            full_path.push_str(parent_path);
            full_path.push('\\');
            full_path.push_str(name);
            full_path.into()
        } else {
            format!("{}:\\{}", self.drive_letter, name).into()
        }
    }
}

/// Convert Windows FILETIME to DateTime
fn filetime_to_datetime(nt_timestamp: u64) -> Option<DateTime<Utc>> {
    // NT timestamps are 100-nanosecond intervals since January 1, 1601
    // Unix epoch is January 1, 1970
    const NT_TO_UNIX_OFFSET: u64 = 116444736000000000;

    if nt_timestamp == 0 || nt_timestamp < NT_TO_UNIX_OFFSET {
        return None;
    }

    let unix_timestamp = nt_timestamp - NT_TO_UNIX_OFFSET;
    let secs = (unix_timestamp / 10_000_000) as i64;
    let nanos = ((unix_timestamp % 10_000_000) * 100) as u32;

    Utc.timestamp_opt(secs, nanos).single()
}

/// Format size in human-readable format
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
