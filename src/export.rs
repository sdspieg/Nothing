use crate::search::SearchResult;
use anyhow::Result;
use std::fs::File;
use std::io::Write;
use serde_json::json;

/// Export search results to CSV format
pub fn export_csv(results: &[SearchResult], path: &str) -> Result<()> {
    let mut file = File::create(path)?;

    // Write CSV header
    writeln!(file, "Name,Path,Type,Size (bytes),Size (formatted),Modified,Created,Accessed,Score")?;

    // Write each result
    for result in results {
        let entry = &result.entry;

        let entry_type = if entry.is_directory { "Directory" } else { "File" };
        let size_formatted = format_file_size(entry.size);
        let modified = entry.modified.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
        let created = entry.created.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
        let accessed = entry.accessed.map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();

        // Escape CSV fields (handle commas, quotes)
        let name = escape_csv_field(&entry.name);
        let path = escape_csv_field(&entry.path);

        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{}",
            name,
            path,
            entry_type,
            entry.size,
            size_formatted,
            modified,
            created,
            accessed,
            result.score
        )?;
    }

    Ok(())
}

/// Export search results to JSON format
pub fn export_json(results: &[SearchResult], path: &str) -> Result<()> {
    let mut file = File::create(path)?;

    let json_results: Vec<_> = results
        .iter()
        .map(|result| {
            json!({
                "name": result.entry.name,
                "path": result.entry.path,
                "type": if result.entry.is_directory { "directory" } else { "file" },
                "size": result.entry.size,
                "size_formatted": format_file_size(result.entry.size),
                "modified": result.entry.modified.map(|d| d.to_rfc3339()),
                "created": result.entry.created.map(|d| d.to_rfc3339()),
                "accessed": result.entry.accessed.map(|d| d.to_rfc3339()),
                "score": result.score,
                "file_id": result.entry.file_id,
                "parent_id": result.entry.parent_id,
            })
        })
        .collect();

    let output = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "total_results": results.len(),
        "results": json_results
    });

    writeln!(file, "{}", serde_json::to_string_pretty(&output)?)?;

    Ok(())
}

/// Escape CSV field (handle commas, quotes, newlines)
fn escape_csv_field(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Format file size in human-readable format
fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[unit_idx])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_csv_field() {
        assert_eq!(escape_csv_field("normal"), "normal");
        assert_eq!(escape_csv_field("has,comma"), "\"has,comma\"");
        assert_eq!(escape_csv_field("has\"quote"), "\"has\"\"quote\"");
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(100), "100 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
    }
}
