use crate::file_entry::FileEntry;
use crate::filters::SearchFilters;
use crate::index::FileIndex;
use nucleo_matcher::{Config, Matcher, Utf32Str};
use nucleo_matcher::pattern::{Pattern, CaseMatching, Normalization};

/// A search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry: FileEntry,
    pub score: u32,
}

/// Search engine for fuzzy file matching
pub struct SearchEngine {
    matcher: Matcher,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            matcher: Matcher::new(Config::DEFAULT),
        }
    }

    /// Search the index with fuzzy matching and optional filters
    /// Returns up to `limit` results sorted by relevance
    pub fn search(&mut self, index: &FileIndex, query: &str, limit: usize) -> Vec<SearchResult> {
        self.search_with_filters(index, query, limit, &SearchFilters::default())
    }

    /// Search with filters
    pub fn search_with_filters(
        &mut self,
        index: &FileIndex,
        query: &str,
        limit: usize,
        filters: &SearchFilters,
    ) -> Vec<SearchResult> {
        if query.is_empty() {
            return Vec::new();
        }

        // Create fuzzy pattern (case-insensitive, with normalization)
        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

        let mut results = Vec::new();

        // Search through all entries
        for entry in index.entries() {
            // Apply filters first
            if !filters.matches(entry) {
                continue;
            }

            // Convert strings to UTF-32 for nucleo
            let mut name_buf = Vec::new();
            let mut path_buf = Vec::new();

            let name_utf32 = Utf32Str::new(&entry.name, &mut name_buf);
            let path_utf32 = Utf32Str::new(&entry.path, &mut path_buf);

            // Try matching against filename first (weighted higher)
            let filename_score = pattern.score(name_utf32, &mut self.matcher);

            // Also try matching against full path
            let path_score = pattern.score(path_utf32, &mut self.matcher);

            // Use the best score, with filename weighted 2x higher
            let best_score = if let Some(name_score) = filename_score {
                Some(name_score.saturating_mul(2))
            } else {
                path_score
            };

            if let Some(score) = best_score {
                results.push(SearchResult {
                    entry: entry.clone(),
                    score,
                });
            }
        }

        // Sort by score (descending - higher is better)
        results.sort_by(|a, b| b.score.cmp(&a.score));

        // Return top N results
        results.truncate(limit);
        results
    }

    /// Get total match count for a query (without limiting results)
    pub fn count_matches(&mut self, index: &FileIndex, query: &str) -> usize {
        self.count_matches_with_filters(index, query, &SearchFilters::default())
    }

    /// Count matches with filters
    pub fn count_matches_with_filters(
        &mut self,
        index: &FileIndex,
        query: &str,
        filters: &SearchFilters,
    ) -> usize {
        if query.is_empty() {
            return 0;
        }

        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut count = 0;

        for entry in index.entries() {
            // Apply filters first
            if !filters.matches(entry) {
                continue;
            }

            let mut name_buf = Vec::new();
            let mut path_buf = Vec::new();

            let name_utf32 = Utf32Str::new(&entry.name, &mut name_buf);
            let path_utf32 = Utf32Str::new(&entry.path, &mut path_buf);

            let filename_score = pattern.score(name_utf32, &mut self.matcher);
            let path_score = pattern.score(path_utf32, &mut self.matcher);

            if filename_score.is_some() || path_score.is_some() {
                count += 1;
            }
        }

        count
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}
