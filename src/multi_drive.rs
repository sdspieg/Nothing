// Multi-drive scanning support
use crate::file_entry::FileEntry;
use crate::index::FileIndex;
use crate::mft_reader::MftReader;
use crate::mft_reader_ntfs::MftReaderNtfs;
use anyhow::Result;
use std::path::PathBuf;

/// Get all available drives on Windows
pub fn get_all_drives() -> Vec<char> {
    let mut drives = Vec::new();

    // Check drives A-Z
    for letter in b'A'..=b'Z' {
        let drive_letter = letter as char;
        let path = format!("{}:\\", drive_letter);

        // Check if drive exists by trying to access it
        if std::path::Path::new(&path).exists() {
            drives.push(drive_letter);
        }
    }

    drives
}

/// Get drive type (Fixed, Removable, Network, etc.)
pub fn get_drive_type(drive: char) -> DriveType {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::GetDriveTypeW;

    let path = format!("{}:\\", drive);
    let wide_path: Vec<u16> = OsStr::new(&path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(wide_path.as_ptr())) };

    // Drive type constants from Windows API
    match drive_type {
        3 => DriveType::Fixed,      // DRIVE_FIXED
        2 => DriveType::Removable,  // DRIVE_REMOVABLE
        4 => DriveType::Network,    // DRIVE_REMOTE
        5 => DriveType::CDRom,      // DRIVE_CDROM
        6 => DriveType::RamDisk,    // DRIVE_RAMDISK
        _ => DriveType::Unknown,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriveType {
    Fixed,
    Removable,
    Network,
    CDRom,
    RamDisk,
    Unknown,
}

/// Check if a path is a cloud storage folder (Google Drive, Dropbox, OneDrive)
pub fn is_cloud_storage_path(path: &str) -> Option<CloudProvider> {
    let lower_path = path.to_lowercase();

    if lower_path.contains("google drive") || lower_path.contains("googledrive") {
        Some(CloudProvider::GoogleDrive)
    } else if lower_path.contains("dropbox") {
        Some(CloudProvider::Dropbox)
    } else if lower_path.contains("onedrive") {
        Some(CloudProvider::OneDrive)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CloudProvider {
    GoogleDrive,
    Dropbox,
    OneDrive,
}

/// Scan all fixed drives
pub fn scan_all_fixed_drives(index: &mut FileIndex, use_full_metadata: bool) -> Result<()> {
    let drives = get_all_drives();

    println!("Found {} drive(s)", drives.len());

    for drive in drives {
        let drive_type = get_drive_type(drive);
        println!("  {}: - {:?}", drive, drive_type);

        // Only scan fixed drives (skip removable, network, etc.)
        if drive_type == DriveType::Fixed {
            println!("\nScanning drive {}:...", drive);

            let result = if use_full_metadata {
                let reader = MftReaderNtfs::new(drive)?;
                reader.scan_into_index(index)
            } else {
                let reader = MftReader::new(drive)?;
                reader.scan_into_index(index)
            };

            match result {
                Ok(_) => println!("✅ Drive {}: scanned successfully", drive),
                Err(e) => eprintln!("⚠️  Drive {}: scan failed: {}", drive, e),
            }
        }
    }

    Ok(())
}

/// Find cloud storage folders on the system
pub fn find_cloud_storage_folders() -> Vec<(CloudProvider, PathBuf)> {
    let mut cloud_folders = Vec::new();

    // Common cloud storage locations
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        let user_path = PathBuf::from(user_profile);

        // Google Drive
        let gdrive_locations = vec![
            user_path.join("Google Drive"),
            user_path.join("GoogleDrive"),
            PathBuf::from("G:\\"), // Sometimes mounted as G:
        ];

        for location in gdrive_locations {
            if location.exists() {
                cloud_folders.push((CloudProvider::GoogleDrive, location));
                break; // Only add once
            }
        }

        // Dropbox
        let dropbox_locations = vec![
            user_path.join("Dropbox"),
            PathBuf::from("D:\\Dropbox"), // Sometimes on D:
        ];

        for location in dropbox_locations {
            if location.exists() {
                cloud_folders.push((CloudProvider::Dropbox, location));
                break;
            }
        }

        // OneDrive
        let onedrive_locations = vec![
            user_path.join("OneDrive"),
            user_path.join("OneDrive - Personal"),
            user_path.join("OneDrive - Business"),
        ];

        for location in onedrive_locations {
            if location.exists() {
                cloud_folders.push((CloudProvider::OneDrive, location.clone()));
            }
        }
    }

    cloud_folders
}

/// Add cloud storage files to the index using directory walking
/// This is slower than MFT scanning but works for any folder
pub fn index_cloud_storage_folder(
    index: &mut FileIndex,
    provider: CloudProvider,
    path: &PathBuf,
) -> Result<usize> {
    use std::fs;
    use walkdir::WalkDir;

    println!("\nIndexing {:?} folder: {:?}", provider, path);

    let mut count = 0;
    let mut skipped = 0;

    for entry in WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path_str = entry.path().to_string_lossy().to_string();

        // Skip cloud-only placeholder files
        if is_placeholder_file(&entry) {
            skipped += 1;
            continue;
        }

        let is_directory = entry.file_type().is_dir();
        let name = entry.file_name().to_string_lossy().to_string();

        // Get metadata
        use chrono::{DateTime, Utc};

        let metadata = fs::metadata(entry.path()).ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let modified = metadata.as_ref().and_then(|m| m.modified().ok());
        let created = metadata.as_ref().and_then(|m| m.created().ok());

        // Convert SystemTime to DateTime<Utc>
        let modified_dt = modified.map(|t| t.into());
        let created_dt = created.map(|t| t.into());

        let file_entry = FileEntry::new(
            name,
            path_str,
            is_directory,
            count as u64, // Fake file_id
            0,            // Fake parent_id
            size,
            modified_dt,
            created_dt,
            None, // No access time from walkdir
        );

        index.add_entry(file_entry);
        count += 1;

        if count % 10000 == 0 {
            println!("  Indexed {} files from {:?}...", count, provider);
        }
    }

    println!(
        "✅ {:?} indexed: {} files ({} cloud-only skipped)",
        provider, count, skipped
    );

    Ok(count)
}

/// Check if a file is a cloud-only placeholder (not actually downloaded)
fn is_placeholder_file(entry: &walkdir::DirEntry) -> bool {
    // Check for cloud placeholder attributes on Windows
    // These files have FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
    // For now, just skip files with size 0 that might be placeholders
    // A more robust solution would use GetFileAttributesW to check for cloud attributes

    if entry.file_type().is_file() {
        if let Ok(metadata) = entry.metadata() {
            // Files with 0 size might be cloud placeholders
            // But also might be legitimately empty files
            // TODO: Use Windows API to check FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
            return false; // For now, don't skip any files
        }
    }

    false
}
