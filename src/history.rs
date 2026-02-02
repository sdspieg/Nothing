use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY: usize = 100;

/// Search history manager
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchHistory {
    /// Recent search queries (most recent last)
    queries: VecDeque<String>,

    /// Current position in history (for arrow key navigation)
    #[serde(skip)]
    position: Option<usize>,

    /// File path for persistence
    #[serde(skip)]
    file_path: PathBuf,
}

impl SearchHistory {
    /// Create a new search history
    pub fn new() -> Result<Self> {
        let file_path = Self::get_history_path()?;

        let history = if file_path.exists() {
            Self::load_from_file(&file_path).unwrap_or_else(|_| Self {
                queries: VecDeque::new(),
                position: None,
                file_path: file_path.clone(),
            })
        } else {
            Self {
                queries: VecDeque::new(),
                position: None,
                file_path: file_path.clone(),
            }
        };

        Ok(history)
    }

    /// Add a query to history
    pub fn add(&mut self, query: &str) {
        let query = query.trim().to_string();

        if query.is_empty() {
            return;
        }

        // Remove duplicate if exists
        self.queries.retain(|q| q != &query);

        // Add to end (most recent)
        self.queries.push_back(query);

        // Trim to max size
        while self.queries.len() > MAX_HISTORY {
            self.queries.pop_front();
        }

        // Reset position
        self.position = None;

        // Save to disk
        let _ = self.save();
    }

    /// Navigate backwards in history (older queries)
    pub fn previous(&mut self, current_query: &str) -> Option<String> {
        if self.queries.is_empty() {
            return None;
        }

        let new_position = if let Some(pos) = self.position {
            if pos > 0 {
                pos - 1
            } else {
                return None; // Already at oldest
            }
        } else {
            // First time pressing up - start from end
            self.queries.len() - 1
        };

        self.position = Some(new_position);
        self.queries.get(new_position).cloned()
    }

    /// Navigate forwards in history (newer queries)
    pub fn next(&mut self) -> Option<String> {
        let pos = self.position?;

        let new_position = pos + 1;

        if new_position >= self.queries.len() {
            // Reached the end - return to empty query
            self.position = None;
            Some(String::new())
        } else {
            self.position = Some(new_position);
            self.queries.get(new_position).cloned()
        }
    }

    /// Reset history navigation position
    pub fn reset_position(&mut self) {
        self.position = None;
    }

    /// Save history to disk
    pub fn save(&self) -> Result<()> {
        let dir = self.file_path.parent().unwrap();
        fs::create_dir_all(dir)?;

        let json = serde_json::to_string_pretty(self)?;
        fs::write(&self.file_path, json)?;

        Ok(())
    }

    /// Load history from file
    fn load_from_file(path: &PathBuf) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let mut history: Self = serde_json::from_str(&contents)?;
        history.file_path = path.clone();
        history.position = None;
        Ok(history)
    }

    /// Get the history file path
    fn get_history_path() -> Result<PathBuf> {
        let username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "default".to_string());

        let path = PathBuf::from(format!("C:\\Users\\{}\\. nothing\\history.json", username));

        Ok(path)
    }

    /// Get all queries (for display)
    pub fn queries(&self) -> &VecDeque<String> {
        &self.queries
    }

    /// Get current position
    pub fn current_position(&self) -> Option<usize> {
        self.position
    }

    /// Clear all history
    pub fn clear(&mut self) -> Result<()> {
        self.queries.clear();
        self.position = None;
        self.save()
    }
}

impl Default for SearchHistory {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            queries: VecDeque::new(),
            position: None,
            file_path: PathBuf::from("history.json"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_query() {
        let mut history = SearchHistory {
            queries: VecDeque::new(),
            position: None,
            file_path: PathBuf::from("test.json"),
        };

        history.add("query1");
        history.add("query2");
        history.add("query1"); // Duplicate

        assert_eq!(history.queries.len(), 2);
        assert_eq!(history.queries.back().unwrap(), "query1");
    }

    #[test]
    fn test_navigation() {
        let mut history = SearchHistory {
            queries: VecDeque::from(vec!["q1".to_string(), "q2".to_string(), "q3".to_string()]),
            position: None,
            file_path: PathBuf::from("test.json"),
        };

        // Navigate backwards
        assert_eq!(history.previous(""), Some("q3".to_string()));
        assert_eq!(history.previous(""), Some("q2".to_string()));
        assert_eq!(history.previous(""), Some("q1".to_string()));
        assert_eq!(history.previous(""), None); // At oldest

        // Navigate forwards
        assert_eq!(history.next(), Some("q2".to_string()));
        assert_eq!(history.next(), Some("q3".to_string()));
        assert_eq!(history.next(), Some(String::new())); // Back to empty
    }
}
