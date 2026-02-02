use chrono::{DateTime, Duration, Utc};
use crate::file_entry::FileEntry;
use anyhow::{Result, anyhow};

/// Search filters for advanced file searching
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    /// Minimum file size in bytes
    pub min_size: Option<u64>,

    /// Maximum file size in bytes
    pub max_size: Option<u64>,

    /// Modified after this date
    pub modified_after: Option<DateTime<Utc>>,

    /// Modified before this date
    pub modified_before: Option<DateTime<Utc>>,

    /// Created after this date
    pub created_after: Option<DateTime<Utc>>,

    /// Created before this date
    pub created_before: Option<DateTime<Utc>>,

    /// File extensions to include (e.g., ["rs", "md"])
    pub extensions: Vec<String>,

    /// Filter by type: Some(true) = dirs only, Some(false) = files only, None = both
    pub is_directory: Option<bool>,
}

impl SearchFilters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse filter from string like "size:>100mb" or "ext:rs,md" or "modified:7d"
    pub fn parse_filter_string(filter_str: &str) -> Result<Self> {
        let mut filters = Self::new();

        // Split by spaces to get individual filters
        for part in filter_str.split_whitespace() {
            if !part.contains(':') {
                continue; // Not a filter, skip
            }

            let parts: Vec<&str> = part.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].to_lowercase();
            let value = parts[1];

            match key.as_str() {
                "size" => {
                    filters.parse_size_filter(value)?;
                }
                "ext" | "extension" => {
                    filters.parse_extension_filter(value)?;
                }
                "modified" | "mod" => {
                    filters.parse_date_filter(value, "modified")?;
                }
                "created" | "cr" => {
                    filters.parse_date_filter(value, "created")?;
                }
                "type" => {
                    filters.parse_type_filter(value)?;
                }
                _ => {} // Ignore unknown filters
            }
        }

        Ok(filters)
    }

    /// Parse size filter like ">100mb", "<1gb", "100kb-500kb"
    fn parse_size_filter(&mut self, value: &str) -> Result<()> {
        if value.contains('-') {
            // Range: "100kb-500kb"
            let parts: Vec<&str> = value.split('-').collect();
            if parts.len() == 2 {
                self.min_size = Some(parse_size(parts[0])?);
                self.max_size = Some(parse_size(parts[1])?);
            }
        } else if value.starts_with('>') {
            // Greater than: ">100mb"
            self.min_size = Some(parse_size(&value[1..])?);
        } else if value.starts_with('<') {
            // Less than: "<1gb"
            self.max_size = Some(parse_size(&value[1..])?);
        } else {
            // Exact or just a number
            let size = parse_size(value)?;
            self.min_size = Some(size);
            self.max_size = Some(size);
        }
        Ok(())
    }

    /// Parse extension filter like "rs" or "rs,md,txt"
    fn parse_extension_filter(&mut self, value: &str) -> Result<()> {
        self.extensions = value
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(())
    }

    /// Parse date filter like "7d" (last 7 days), "2024-01-01", ">2024-01-01"
    fn parse_date_filter(&mut self, value: &str, field: &str) -> Result<()> {
        let date = if value.ends_with('d') || value.ends_with('D') {
            // Relative days: "7d" = last 7 days
            let days: i64 = value[..value.len()-1].parse()
                .map_err(|_| anyhow!("Invalid day count: {}", value))?;
            Utc::now() - Duration::days(days)
        } else if value.starts_with('>') {
            // After date: ">2024-01-01"
            parse_date(&value[1..])?
        } else if value.starts_with('<') {
            // Before date: "<2024-01-01"
            parse_date(&value[1..])?
        } else {
            // Exact date
            parse_date(value)?
        };

        match field {
            "modified" => {
                if value.starts_with('>') || value.ends_with('d') {
                    self.modified_after = Some(date);
                } else if value.starts_with('<') {
                    self.modified_before = Some(date);
                } else {
                    self.modified_after = Some(date);
                }
            }
            "created" => {
                if value.starts_with('>') || value.ends_with('d') {
                    self.created_after = Some(date);
                } else if value.starts_with('<') {
                    self.created_before = Some(date);
                } else {
                    self.created_after = Some(date);
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Parse type filter: "dir", "file"
    fn parse_type_filter(&mut self, value: &str) -> Result<()> {
        match value.to_lowercase().as_str() {
            "dir" | "directory" | "folder" => {
                self.is_directory = Some(true);
            }
            "file" => {
                self.is_directory = Some(false);
            }
            _ => {
                return Err(anyhow!("Invalid type filter: {}. Use 'dir' or 'file'", value));
            }
        }
        Ok(())
    }

    /// Check if a file entry matches all filters
    pub fn matches(&self, entry: &FileEntry) -> bool {
        // Size filters
        if let Some(min_size) = self.min_size {
            if entry.size < min_size {
                return false;
            }
        }

        if let Some(max_size) = self.max_size {
            if entry.size > max_size {
                return false;
            }
        }

        // Modified date filters
        if let Some(after) = self.modified_after {
            if let Some(modified) = entry.modified {
                if modified < after {
                    return false;
                }
            } else {
                return false; // No modified date, can't match
            }
        }

        if let Some(before) = self.modified_before {
            if let Some(modified) = entry.modified {
                if modified > before {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Created date filters
        if let Some(after) = self.created_after {
            if let Some(created) = entry.created {
                if created < after {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(before) = self.created_before {
            if let Some(created) = entry.created {
                if created > before {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Extension filters
        if !self.extensions.is_empty() && !entry.is_directory {
            let entry_ext = entry.path
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_lowercase();

            if !self.extensions.contains(&entry_ext) {
                return false;
            }
        }

        // Type filter
        if let Some(is_dir) = self.is_directory {
            if entry.is_directory != is_dir {
                return false;
            }
        }

        true
    }

    /// Check if any filters are active
    pub fn is_empty(&self) -> bool {
        self.min_size.is_none()
            && self.max_size.is_none()
            && self.modified_after.is_none()
            && self.modified_before.is_none()
            && self.created_after.is_none()
            && self.created_before.is_none()
            && self.extensions.is_empty()
            && self.is_directory.is_none()
    }

    /// Get a human-readable description of active filters
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();

        if let Some(min) = self.min_size {
            parts.push(format!("size ≥ {}", format_size(min)));
        }

        if let Some(max) = self.max_size {
            parts.push(format!("size ≤ {}", format_size(max)));
        }

        if let Some(date) = self.modified_after {
            parts.push(format!("modified after {}", date.format("%Y-%m-%d")));
        }

        if let Some(date) = self.modified_before {
            parts.push(format!("modified before {}", date.format("%Y-%m-%d")));
        }

        if !self.extensions.is_empty() {
            parts.push(format!("ext: {}", self.extensions.join(", ")));
        }

        if let Some(is_dir) = self.is_directory {
            parts.push(if is_dir { "directories only" } else { "files only" }.to_string());
        }

        if parts.is_empty() {
            "no filters".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Parse size string like "100kb", "1.5gb", "500mb"
fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim().to_lowercase();

    // Extract number and unit
    let (num_str, unit) = if s.ends_with("kb") {
        (&s[..s.len()-2], 1024u64)
    } else if s.ends_with("mb") {
        (&s[..s.len()-2], 1024u64 * 1024)
    } else if s.ends_with("gb") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024)
    } else if s.ends_with("tb") {
        (&s[..s.len()-2], 1024u64 * 1024 * 1024 * 1024)
    } else if s.ends_with('b') {
        (&s[..s.len()-1], 1u64)
    } else {
        // No unit, assume bytes
        (s.as_str(), 1u64)
    };

    let num: f64 = num_str.trim().parse()
        .map_err(|_| anyhow!("Invalid size number: {}", num_str))?;

    Ok((num * unit as f64) as u64)
}

/// Parse date string like "2024-01-01"
fn parse_date(s: &str) -> Result<DateTime<Utc>> {
    use chrono::NaiveDate;

    let date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow!("Invalid date format: {}. Use YYYY-MM-DD", s))?;

    Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc())
}

/// Format size for display
fn format_size(bytes: u64) -> String {
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
    fn test_parse_size() {
        assert_eq!(parse_size("100").unwrap(), 100);
        assert_eq!(parse_size("1kb").unwrap(), 1024);
        assert_eq!(parse_size("1.5mb").unwrap(), 1572864);
        assert_eq!(parse_size("2gb").unwrap(), 2147483648);
    }

    #[test]
    fn test_parse_filter_string() {
        let filters = SearchFilters::parse_filter_string("size:>100mb ext:rs,md type:file").unwrap();
        assert_eq!(filters.min_size, Some(104857600));
        assert_eq!(filters.extensions, vec!["rs", "md"]);
        assert_eq!(filters.is_directory, Some(false));
    }
}
