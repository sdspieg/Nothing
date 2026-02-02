use std::path::Path;
use nothing::persistence;

fn main() -> anyhow::Result<()> {
    println!("========================================================================");
    println!("          NOTHING - Drive Indexing Status Monitor                ");
    println!("========================================================================");
    println!();

    // Check if a scan is currently running
    let scan_running = std::process::Command::new("tasklist")
        .output()
        .ok()
        .and_then(|output| {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("nothing.exe") {
                // Extract memory usage
                for line in output_str.lines() {
                    if line.contains("nothing.exe") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 5 {
                            let mem_kb = parts[4].replace(",", "");
                            if let Ok(kb) = mem_kb.parse::<u64>() {
                                return Some(kb);
                            }
                        }
                    }
                }
            }
            None
        });

    if let Some(mem_kb) = scan_running {
        let mem_gb = mem_kb as f64 / 1_048_576.0;
        println!("[SCAN] ACTIVE SCAN DETECTED:");
        println!("   Memory usage: {:.2} GB", mem_gb);
        println!("   Status: Building index in memory (will write when complete)");
        println!();
    }

    // Get all available drives (A-Z)
    let all_drives: Vec<char> = ('A'..='Z').collect();

    let mut total_indexed = 0;
    let mut total_drives = 0;
    let mut total_files = 0;
    let mut total_size_bytes: u64 = 0;

    println!("{:<8} {:<20} {:<15} {:<20} {:<15}",
             "Drive", "Status", "Index Size", "Files", "Directories");
    println!("{}", "-".repeat(80));

    for drive in all_drives {
        // Check if drive exists
        let drive_path = format!("{}:\\", drive);

        if !Path::new(&drive_path).exists() {
            continue;
        }

        total_drives += 1;

        // Check if index exists
        match persistence::get_index_path(drive) {
            Ok(index_path) => {
                if Path::new(&index_path).exists() {
                    // Index exists - load it to get stats
                    match persistence::load_index(&index_path) {
                        Ok(index) => {
                            let metadata = std::fs::metadata(&index_path)?;
                            let size_mb = metadata.len() as f64 / 1_048_576.0;

                            total_indexed += 1;
                            total_files += index.file_count();
                            total_size_bytes += metadata.len();

                            println!("{:<8} {:<20} {:<15} {:<20} {:<15}",
                                     format!("{}:", drive),
                                     "[OK] INDEXED",
                                     format!("{:.1} MB", size_mb),
                                     index.file_count().to_string(),
                                     index.directory_count().to_string());
                        }
                        Err(_) => {
                            println!("{:<8} {:<20} {:<15} {:<20} {:<15}",
                                     format!("{}:", drive),
                                     "[ERR] CORRUPT",
                                     "-",
                                     "-",
                                     "-");
                        }
                    }
                } else {
                    let status = if scan_running.is_some() && total_indexed > 0 {
                        "[SCAN] SCANNING"
                    } else {
                        "[WAIT] WAITING"
                    };

                    println!("{:<8} {:<20} {:<15} {:<20} {:<15}",
                             format!("{}:", drive),
                             status,
                             "-",
                             "-",
                             "-");
                }
            }
            Err(_) => {
                println!("{:<8} {:<20} {:<15} {:<20} {:<15}",
                         format!("{}:", drive),
                         "[WAIT] WAITING",
                         "-",
                         "-",
                         "-");
            }
        }
    }

    println!("{}", "-".repeat(80));
    println!();

    let coverage_pct = if total_drives > 0 {
        (total_indexed as f64 / total_drives as f64) * 100.0
    } else {
        0.0
    };

    println!("== SUMMARY ==");
    println!("   Total drives detected: {}", total_drives);
    println!("   Drives indexed: {} / {} ({:.1}%)",
             total_indexed, total_drives, coverage_pct);
    println!("   Total files indexed: {}", total_files);
    println!("   Total index size: {:.1} GB", total_size_bytes as f64 / 1_073_741_824.0);
    println!();

    if total_indexed < total_drives {
        println!("** {} drive(s) still need indexing!", total_drives - total_indexed);
        println!("   Run: nothing.exe -a -f");
    } else {
        println!("** All drives are indexed!");
    }

    Ok(())
}
