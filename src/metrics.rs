use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Performance metrics for search operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMetrics {
    /// Total number of searches performed
    pub total_searches: usize,

    /// Total search time across all searches
    pub total_search_time: Duration,

    /// Total matches found across all searches
    pub total_matches: usize,

    /// Number of searches with results
    pub searches_with_results: usize,

    /// Number of searches with no results
    pub searches_with_no_results: usize,

    /// Fastest search time
    pub fastest_search: Option<Duration>,

    /// Slowest search time
    pub slowest_search: Option<Duration>,

    /// Most matches in a single search
    pub most_matches: usize,

    /// Current session start time (not serialized)
    #[serde(skip)]
    session_start: Option<Instant>,
}

impl SearchMetrics {
    pub fn new() -> Self {
        Self {
            total_searches: 0,
            total_search_time: Duration::ZERO,
            total_matches: 0,
            searches_with_results: 0,
            searches_with_no_results: 0,
            fastest_search: None,
            slowest_search: None,
            most_matches: 0,
            session_start: Some(Instant::now()),
        }
    }

    /// Record a search operation
    pub fn record_search(&mut self, duration: Duration, match_count: usize) {
        self.total_searches += 1;
        self.total_search_time += duration;
        self.total_matches += match_count;

        if match_count > 0 {
            self.searches_with_results += 1;
        } else {
            self.searches_with_no_results += 1;
        }

        // Update fastest
        if self.fastest_search.is_none() || duration < self.fastest_search.unwrap() {
            self.fastest_search = Some(duration);
        }

        // Update slowest
        if self.slowest_search.is_none() || duration > self.slowest_search.unwrap() {
            self.slowest_search = Some(duration);
        }

        // Update most matches
        if match_count > self.most_matches {
            self.most_matches = match_count;
        }
    }

    /// Get average search time
    pub fn average_search_time(&self) -> Duration {
        if self.total_searches == 0 {
            Duration::ZERO
        } else {
            self.total_search_time / self.total_searches as u32
        }
    }

    /// Get average matches per search
    pub fn average_matches(&self) -> f64 {
        if self.total_searches == 0 {
            0.0
        } else {
            self.total_matches as f64 / self.total_searches as f64
        }
    }

    /// Get success rate (searches with results / total searches)
    pub fn success_rate(&self) -> f64 {
        if self.total_searches == 0 {
            0.0
        } else {
            self.searches_with_results as f64 / self.total_searches as f64
        }
    }

    /// Get session duration
    pub fn session_duration(&self) -> Duration {
        if let Some(start) = self.session_start {
            start.elapsed()
        } else {
            Duration::ZERO
        }
    }

    /// Format metrics for display
    pub fn format_summary(&self) -> String {
        if self.total_searches == 0 {
            return "No searches performed yet".to_string();
        }

        format!(
            "Search Statistics:\n\
             • Total searches: {}\n\
             • Searches with results: {} ({:.1}%)\n\
             • Total matches: {}\n\
             • Average matches: {:.1}\n\
             • Average search time: {:.2}ms\n\
             • Fastest search: {:.2}ms\n\
             • Slowest search: {:.2}ms\n\
             • Most matches: {}\n\
             • Session duration: {}",
            self.total_searches,
            self.searches_with_results,
            self.success_rate() * 100.0,
            self.total_matches,
            self.average_matches(),
            self.average_search_time().as_secs_f64() * 1000.0,
            self.fastest_search.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0),
            self.slowest_search.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0),
            self.most_matches,
            format_duration(self.session_duration())
        )
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for SearchMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Format duration in human-readable format
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_search() {
        let mut metrics = SearchMetrics::new();

        metrics.record_search(Duration::from_millis(10), 5);
        metrics.record_search(Duration::from_millis(20), 10);
        metrics.record_search(Duration::from_millis(5), 0);

        assert_eq!(metrics.total_searches, 3);
        assert_eq!(metrics.searches_with_results, 2);
        assert_eq!(metrics.searches_with_no_results, 1);
        assert_eq!(metrics.total_matches, 15);
        assert_eq!(metrics.fastest_search, Some(Duration::from_millis(5)));
        assert_eq!(metrics.slowest_search, Some(Duration::from_millis(20)));
        assert_eq!(metrics.most_matches, 10);
    }

    #[test]
    fn test_averages() {
        let mut metrics = SearchMetrics::new();

        metrics.record_search(Duration::from_millis(10), 5);
        metrics.record_search(Duration::from_millis(20), 15);

        assert_eq!(metrics.average_search_time(), Duration::from_millis(15));
        assert_eq!(metrics.average_matches(), 10.0);
        assert_eq!(metrics.success_rate(), 1.0);
    }
}
