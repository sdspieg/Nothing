mod error;
mod file_entry;
mod index;
mod interactive;
mod mft_reader;
mod mft_reader_ntfs;
mod search;

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Create file index
    let mut index = FileIndex::new();

    // Choose scanner based on mode
    if args.full_metadata {
        println!("Using full metadata mode (includes sizes and timestamps)");
        let reader = MftReaderNtfs::new(args.drive)?;
        reader.scan_into_index(&mut index)?;
    } else {
        println!("Using fast mode (names and paths only)");
        let reader = MftReader::new(args.drive)?;
        reader.scan_into_index(&mut index)?;
    }

    // Enter interactive mode if requested
    if args.interactive {
        println!("\nEntering interactive search mode...\n");
        interactive::run_interactive_search(&index)?;
    }

    Ok(())
}
