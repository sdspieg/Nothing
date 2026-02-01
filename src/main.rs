mod error;
mod file_entry;
mod index;
mod interactive;
mod mft_reader;
mod mft_reader_ntfs;
mod multi_drive;
mod search;
mod sector_aligned_reader;
mod volume_test;

use anyhow::Result;
use clap::Parser;
use index::FileIndex;
use mft_reader::MftReader;
use mft_reader_ntfs::MftReaderNtfs;

#[derive(Parser, Debug)]
#[command(name = "nothing")]
#[command(author = "Nothing Team")]
#[command(version = "0.2.0")]
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
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Run volume tests if requested
    if args.test_volume {
        volume_test::run_all_tests(args.drive);
        return Ok(());
    }

    // Create file index
    let mut index = FileIndex::new();

    // Scan drives based on flags
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

    // Enter interactive mode if requested
    if args.interactive {
        println!("\nEntering interactive search mode...\n");
        interactive::run_interactive_search(&index)?;
    }

    Ok(())
}
