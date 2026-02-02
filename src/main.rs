mod cloud_monitor;
mod error;
mod export;
mod file_entry;
mod filters;
mod gui;
mod history;
mod index;
mod interactive;
mod metrics;
mod mft_reader;
mod mft_reader_ntfs;
mod multi_drive;
mod persistence;
mod search;
mod sector_aligned_reader;
mod usn_monitor;
mod volume_test;

use anyhow::Result;
use clap::Parser;
use index::FileIndex;
use mft_reader::MftReader;
use mft_reader_ntfs::MftReaderNtfs;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(name = "nothing")]
#[command(author = "Nothing Team")]
#[command(version = "0.6.1")]
#[command(about = "Fast Windows file search tool - reads NTFS MFT directly", long_about = None)]
struct Args {
    /// Drive letter to scan (e.g., C, D, E)
    #[arg(default_value = "C")]
    drive: char,

    /// Start interactive search mode after scanning
    #[arg(short, long)]
    interactive: bool,

    /// Use full metadata mode (slower but includes file sizes and timestamps)
    #[arg(short = 'f', long)]
    full_metadata: bool,

    /// Run volume access tests
    #[arg(long)]
    test_volume: bool,

    /// Scan all fixed drives (not just one)
    #[arg(short = 'a', long)]
    all_drives: bool,

    /// Include cloud storage folders (Google Drive, Dropbox, OneDrive)
    #[arg(short = 'c', long)]
    include_cloud: bool,

    /// Launch GUI mode instead of CLI
    #[arg(short = 'g', long)]
    gui: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Run volume tests if requested
    if args.test_volume {
        volume_test::run_all_tests(args.drive);
        return Ok(());
    }

    // Determine which drives to monitor
    let drives: Vec<char> = if args.all_drives {
        multi_drive::get_all_drives()
    } else {
        vec![args.drive]
    };

    // Try to load cached indexes for all available drives
    let mut index = FileIndex::new();

    // Load indexes for all NTFS drives with cached indexes
    let available_drives: Vec<char> = multi_drive::get_all_drives();
    let mut loaded_count = 0;
    let mut total_files = 0;
    let mut total_dirs = 0;

    for drive in &available_drives {
        if let Ok(cache_path) = persistence::get_index_path(*drive) {
            if std::path::Path::new(&cache_path).exists() {
                match persistence::load_index(&cache_path) {
                    Ok(drive_index) => {
                        let files = drive_index.file_count();
                        let dirs = drive_index.directory_count();
                        println!("✅ Loaded {} drive: {} files, {} directories", drive, files, dirs);

                        // Merge this drive's index into the main index
                        for entry in drive_index.entries() {
                            index.add_entry(entry.clone());
                        }

                        loaded_count += 1;
                        total_files += files;
                        total_dirs += dirs;
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to load {} drive index: {}", drive, e);
                    }
                }
            }
        }
    }

    if loaded_count > 0 {
        println!("\n📊 Combined Index: {} drives, {} total files, {} directories",
                 loaded_count, total_files, total_dirs);
    } else {
        println!("No cached indexes found.");
    }

    // If index is empty, scan drives
    if index.is_empty() {
        if args.all_drives {
            println!("Scanning all fixed drives...\n");
            multi_drive::scan_all_fixed_drives(&mut index, args.full_metadata)?;
        } else {
            // Single drive mode
            if args.full_metadata {
                println!("Using full metadata mode (includes sizes and timestamps)");
                let reader = MftReaderNtfs::new(args.drive)?;
                reader.scan_into_index(&mut index)?;
            } else {
                println!("Using fast mode (names and paths only)");
                let reader = MftReader::new(args.drive)?;
                reader.scan_into_index(&mut index)?;
            }
        }

        // Save index for next time
        if !drives.is_empty() {
            println!("\nSaving index to disk...");
            let cache_path = persistence::get_index_path(drives[0])?;
            persistence::save_index(&index, &cache_path)?;
            println!("✅ Index saved");
        }
    }

    // Add cloud storage if requested
    if args.include_cloud {
        println!("\n=== Scanning Cloud Storage ===");
        let cloud_folders = multi_drive::find_cloud_storage_folders();

        if cloud_folders.is_empty() {
            println!("No cloud storage folders found.");
        } else {
            for (provider, path) in cloud_folders {
                match multi_drive::index_cloud_storage_folder(&mut index, provider, &path) {
                    Ok(count) => println!("Added {} files from {:?}", count, provider),
                    Err(e) => eprintln!("Failed to index {:?}: {}", provider, e),
                }
            }
        }
    }

    // Enter interactive mode (CLI or GUI) with monitoring if requested
    if args.interactive || args.gui {
        // Wrap index in Arc<Mutex<>> for thread-safe access
        let index_arc = Arc::new(Mutex::new(index));
        let index_clone = Arc::clone(&index_arc);

        // Start USN monitoring for NTFS drives
        let monitor = usn_monitor::UsnMonitor::new(drives.clone(), Arc::clone(&index_arc))?;

        // Start cloud monitoring if requested
        let _cloud_monitor = if args.include_cloud {
            let cloud_folders: Vec<std::path::PathBuf> = multi_drive::find_cloud_storage_folders()
                .into_iter()
                .map(|(_, path)| path)
                .collect();

            if !cloud_folders.is_empty() {
                Some(cloud_monitor::CloudMonitor::new(cloud_folders, Arc::clone(&index_arc))?)
            } else {
                None
            }
        } else {
            None
        };

        // Run interactive search (GUI or CLI)
        if args.gui {
            println!("\nLaunching GUI...\n");
            let gui_index = Arc::clone(&index_clone);
            gui::run(gui_index)?;
        } else {
            println!("\nEntering interactive search mode with real-time monitoring...\n");
            interactive::run_interactive_search_with_arc(&index_clone)?;
        }

        // Stop monitoring
        monitor.stop();

        // Save updated index
        if !drives.is_empty() {
            println!("\nSaving updated index...");
            let final_index = index_clone.lock().unwrap();
            let cache_path = persistence::get_index_path(drives[0])?;
            persistence::save_index(&*final_index, &cache_path)?;
            println!("✅ Index saved");
        }
    }

    Ok(())
}
