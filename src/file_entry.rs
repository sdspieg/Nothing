use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a file or directory entry from the MFT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File name (without path)
    pub name: String,

    /// Full path to the file
    pub path: String,

    /// Whether this is a directory
    pub is_directory: bool,

    /// MFT file reference number
    pub file_id: u64,

    /// Parent file reference number
    pub parent_id: u64,

    /// File size in bytes (0 for directories)
    pub size: u64,

    /// Last modified timestamp
    pub modified: Option<DateTime<Utc>>,

    /// Created timestamp
    pub created: Option<DateTime<Utc>>,

    /// Last accessed timestamp
    pub accessed: Option<DateTime<Utc>>,
}

impl FileEntry {
    /// Create a new FileEntry
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        path: String,
        is_directory: bool,
        file_id: u64,
        parent_id: u64,
        size: u64,
        modified: Option<DateTime<Utc>>,
        created: Option<DateTime<Utc>>,
        accessed: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            name,
            path,
            is_directory,
            file_id,
            parent_id,
            size,
            modified,
            created,
            accessed,
        }
    }
}
