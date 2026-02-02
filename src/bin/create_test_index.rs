use chrono::Utc;
use nothing::file_entry::FileEntry;
use nothing::index::FileIndex;
use nothing::persistence;
use std::sync::{Arc, Mutex};

fn main() -> anyhow::Result<()> {
    // CRITICAL SAFETY CHECK: Prevent overwriting real indexes!
    let cache_path = persistence::get_index_path('C')?;

    if std::path::Path::new(&cache_path).exists() {
        let metadata = std::fs::metadata(&cache_path)?;

        // If the index is larger than 100 KB, it's probably a real index with actual data
        if metadata.len() > 100_000 {
            eprintln!("\n❌ ERROR: A real index file already exists!");
            eprintln!("   Location: {}", cache_path);
            eprintln!("   Size: {:.2} MB", metadata.len() as f64 / 1_024_000.0);
            eprintln!("\n   This tool is ONLY for creating test data.");
            eprintln!("   Running it would DESTROY your real index with {} files!",
                     "millions of");
            eprintln!("\n   To create test data, first delete the existing index:");
            eprintln!("   rm {}", cache_path);
            eprintln!("\n   ABORTING to protect your data.");
            std::process::exit(1);
        }
    }

    println!("Creating test index with sample data...");

    let mut index = FileIndex::new();

    // Add some sample files for testing
    let sample_files = vec![
        ("README.md", "C:\\Apps\\Nothing\\README.md", false, 12345),
        ("Cargo.toml", "C:\\Apps\\Nothing\\Cargo.toml", false, 2300),
        ("main.rs", "C:\\Apps\\Nothing\\src\\main.rs", false, 45200),
        ("index.rs", "C:\\Apps\\Nothing\\src\\index.rs", false, 8900),
        ("search.rs", "C:\\Apps\\Nothing\\src\\search.rs", false, 5600),
        ("interactive.rs", "C:\\Apps\\Nothing\\src\\interactive.rs", false, 15200),
        ("filters.rs", "C:\\Apps\\Nothing\\src\\filters.rs", false, 12000),
        ("export.rs", "C:\\Apps\\Nothing\\src\\export.rs", false, 6700),
        ("history.rs", "C:\\Apps\\Nothing\\src\\history.rs", false, 8900),
        ("metrics.rs", "C:\\Apps\\Nothing\\src\\metrics.rs", false, 7800),
        ("src", "C:\\Apps\\Nothing\\src", true, 0),
        ("target", "C:\\Apps\\Nothing\\target", true, 0),
        ("CHANGELOG.md", "C:\\Apps\\Nothing\\CHANGELOG.md", false, 45000),
        ("PHASE4_SUMMARY.md", "C:\\Apps\\Nothing\\PHASE4_SUMMARY.md", false, 23000),
        ("PHASE5_PLAN.md", "C:\\Apps\\Nothing\\PHASE5_PLAN.md", false, 18000),
        ("app.rs", "C:\\Apps\\Nothing\\src\\gui\\app.rs", false, 25000),
        ("theme.rs", "C:\\Apps\\Nothing\\src\\gui\\theme.rs", false, 3200),
        ("gui", "C:\\Apps\\Nothing\\src\\gui", true, 0),
        ("widgets", "C:\\Apps\\Nothing\\src\\gui\\widgets", true, 0),
        ("dialogs", "C:\\Apps\\Nothing\\src\\gui\\dialogs", true, 0),
        ("video.mp4", "C:\\Videos\\sample_video.mp4", false, 150000000),
        ("large_file.zip", "C:\\Downloads\\large_file.zip", false, 500000000),
        ("test.rs", "C:\\Code\\test.rs", false, 4500),
        ("config.json", "C:\\AppData\\config.json", false, 2300),
        ("photo.jpg", "C:\\Pictures\\photo.jpg", false, 3400000),
        ("document.pdf", "C:\\Documents\\document.pdf", false, 1200000),
        ("music.mp3", "C:\\Music\\song.mp3", false, 5600000),
        ("data.csv", "C:\\Data\\export.csv", false, 890000),
        ("notes.txt", "C:\\Notes\\meeting_notes.txt", false, 15000),
        ("script.py", "C:\\Scripts\\automation.py", false, 8900),
    ];

    let now = Utc::now();

    for (idx, (name, path, is_dir, size)) in sample_files.iter().enumerate() {
        let entry = FileEntry::new(
            name.to_string(),
            path.to_string(),
            *is_dir,
            idx as u64 + 1000,      // file_id
            0,                       // parent_id
            *size,
            Some(now - chrono::Duration::days(idx as i64 % 30)),  // modified
            Some(now - chrono::Duration::days(idx as i64 % 60)),  // created
            Some(now - chrono::Duration::days(idx as i64 % 15)),  // accessed
        );
        index.add_entry(entry);
    }

    println!("Created index with {} files", index.file_count());

    // Save to the expected location
    let cache_path = persistence::get_index_path('C')?;
    persistence::save_index(&index, &cache_path)?;

    println!("✅ Test index saved to: {}", cache_path);
    println!("Now you can run: nothing.exe --gui");

    Ok(())
}
