use nothing::persistence;

fn main() -> anyhow::Result<()> {
    let cache_path = persistence::get_index_path('C')?;

    println!("Reading index from: {}\n", cache_path);

    let index = persistence::load_index(&cache_path)?;

    println!("📊 Index Statistics:");
    println!("   Total entries: {}", index.len());
    println!("   Files: {}", index.file_count());
    println!("   Directories: {}", index.directory_count());
    println!("   Memory usage: ~{:.2} KB\n", index.memory_usage() as f64 / 1024.0);

    println!("📁 Sample entries (first 30):");
    println!("{:<40} {:<15} {:<60}", "Name", "Type", "Path");
    println!("{}", "-".repeat(115));

    for (i, entry) in index.entries().iter().take(30).enumerate() {
        let entry_type = if entry.is_directory { "📁 DIR" } else { "📄 FILE" };
        let size_str = if entry.is_directory {
            String::new()
        } else {
            format!("({} bytes)", entry.size)
        };

        println!(
            "{:<40} {:<15} {}",
            entry.name,
            entry_type,
            entry.path
        );

        if !size_str.is_empty() {
            println!("    └─ {}", size_str);
        }
    }

    if index.len() > 30 {
        println!("\n... and {} more entries", index.len() - 30);
    }

    Ok(())
}
