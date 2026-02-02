use crate::export;
use crate::filters::SearchFilters;
use crate::history::SearchHistory;
use crate::index::FileIndex;
use crate::metrics::SearchMetrics;
use crate::search::SearchEngine;
use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use std::io::{stdout, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

const RESULT_LIMIT: usize = 50;

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

/// Run interactive search mode with Arc<Mutex<>> index (for monitoring)
pub fn run_interactive_search_with_arc(index: &Arc<Mutex<FileIndex>>) -> Result<()> {
    let mut stdout = stdout();
    let mut search_engine = SearchEngine::new();
    let mut query = String::new();
    let mut history = SearchHistory::new()?;
    let mut metrics = SearchMetrics::new();
    let mut show_help = false;
    let mut show_stats = false;
    let mut last_results = Vec::new();

    // Enable raw mode for character-by-character input
    terminal::enable_raw_mode()?;

    // Clear screen and show initial prompt
    display_header(&mut stdout, show_help)?;

    loop {
        // Read keyboard events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                match handle_key_event(key_event, &mut query, &mut history, &mut show_help, &mut show_stats)? {
                    KeyAction::Exit => break,
                    KeyAction::UpdateSearch => {
                        // Lock index for searching
                        let index_guard = index.lock().unwrap();
                        let start = Instant::now();
                        let results = perform_search(&mut search_engine, &*index_guard, &query)?;
                        let duration = start.elapsed();
                        metrics.record_search(duration, results.len());
                        last_results = results.clone();
                        drop(index_guard); // Release lock

                        display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                    }
                    KeyAction::Export => {
                        // Export current results
                        if !last_results.is_empty() {
                            export_results(&mut stdout, &last_results)?;
                            // Redisplay after export
                            let index_guard = index.lock().unwrap();
                            let start = Instant::now();
                            let results = perform_search(&mut search_engine, &*index_guard, &query)?;
                            let duration = start.elapsed();
                            drop(index_guard);
                            display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                        }
                    }
                    KeyAction::ToggleHelp => {
                        show_help = !show_help;
                        let index_guard = index.lock().unwrap();
                        let start = Instant::now();
                        let results = perform_search(&mut search_engine, &*index_guard, &query)?;
                        let duration = start.elapsed();
                        drop(index_guard);
                        display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                    }
                    KeyAction::ToggleStats => {
                        show_stats = !show_stats;
                        let index_guard = index.lock().unwrap();
                        let start = Instant::now();
                        let results = perform_search(&mut search_engine, &*index_guard, &query)?;
                        let duration = start.elapsed();
                        drop(index_guard);
                        display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                    }
                    KeyAction::None => {}
                }
            }
        }
    }

    // Save search history before exit
    history.save()?;

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    println!("Goodbye!");

    Ok(())
}

/// Run interactive search mode
pub fn run_interactive_search(index: &FileIndex) -> Result<()> {
    let mut stdout = stdout();
    let mut search_engine = SearchEngine::new();
    let mut query = String::new();
    let mut history = SearchHistory::new()?;
    let mut metrics = SearchMetrics::new();
    let mut show_help = false;
    let mut show_stats = false;
    let mut last_results = Vec::new();

    // Enable raw mode for character-by-character input
    terminal::enable_raw_mode()?;

    // Clear screen and show initial prompt
    display_header(&mut stdout, show_help)?;

    loop {
        // Read keyboard events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                match handle_key_event(key_event, &mut query, &mut history, &mut show_help, &mut show_stats)? {
                    KeyAction::Exit => break,
                    KeyAction::UpdateSearch => {
                        let start = Instant::now();
                        let results = perform_search(&mut search_engine, index, &query)?;
                        let duration = start.elapsed();
                        metrics.record_search(duration, results.len());
                        last_results = results.clone();
                        display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                    }
                    KeyAction::Export => {
                        // Export current results
                        if !last_results.is_empty() {
                            export_results(&mut stdout, &last_results)?;
                            // Redisplay after export
                            let start = Instant::now();
                            let results = perform_search(&mut search_engine, index, &query)?;
                            let duration = start.elapsed();
                            display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                        }
                    }
                    KeyAction::ToggleHelp => {
                        show_help = !show_help;
                        let start = Instant::now();
                        let results = perform_search(&mut search_engine, index, &query)?;
                        let duration = start.elapsed();
                        display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                    }
                    KeyAction::ToggleStats => {
                        show_stats = !show_stats;
                        let start = Instant::now();
                        let results = perform_search(&mut search_engine, index, &query)?;
                        let duration = start.elapsed();
                        display_search_results(&mut stdout, &results, &query, duration, show_help, show_stats, &metrics)?;
                    }
                    KeyAction::None => {}
                }
            }
        }
    }

    // Save search history before exit
    history.save()?;

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    println!("Goodbye!");

    Ok(())
}

enum KeyAction {
    Exit,
    UpdateSearch,
    Export,
    ToggleHelp,
    ToggleStats,
    None,
}

/// Handle keyboard input
fn handle_key_event(
    key: KeyEvent,
    query: &mut String,
    history: &mut SearchHistory,
    show_help: &mut bool,
    show_stats: &mut bool,
) -> Result<KeyAction> {
    match key.code {
        // Exit on Ctrl+C or Ctrl+D
        KeyCode::Char('c') | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Ok(KeyAction::Exit)
        }

        // Export on Ctrl+E
        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Ok(KeyAction::Export)
        }

        // Toggle help on F1
        KeyCode::F(1) => {
            Ok(KeyAction::ToggleHelp)
        }

        // Toggle stats on F2
        KeyCode::F(2) => {
            Ok(KeyAction::ToggleStats)
        }

        // Backspace
        KeyCode::Backspace => {
            query.pop();
            history.reset_position();
            Ok(KeyAction::UpdateSearch)
        }

        // Arrow Up - navigate history backwards
        KeyCode::Up => {
            if let Some(prev_query) = history.previous(query) {
                *query = prev_query;
                Ok(KeyAction::UpdateSearch)
            } else {
                Ok(KeyAction::None)
            }
        }

        // Arrow Down - navigate history forwards
        KeyCode::Down => {
            if let Some(next_query) = history.next() {
                *query = next_query;
                Ok(KeyAction::UpdateSearch)
            } else {
                Ok(KeyAction::None)
            }
        }

        // Regular character input
        KeyCode::Char(c) => {
            query.push(c);
            history.reset_position();
            Ok(KeyAction::UpdateSearch)
        }

        // Enter - add to history and update
        KeyCode::Enter => {
            if !query.is_empty() {
                history.add(query);
            }
            Ok(KeyAction::UpdateSearch)
        }

        // Ignore other keys
        _ => Ok(KeyAction::None),
    }
}

/// Perform search with filter parsing
fn perform_search(
    search_engine: &mut SearchEngine,
    index: &FileIndex,
    query_str: &str,
) -> Result<Vec<crate::search::SearchResult>> {
    // Parse filters from query
    let filters = SearchFilters::parse_filter_string(query_str).unwrap_or_default();

    // Extract the actual search query (remove filter parts)
    let search_query = extract_search_query(query_str);

    // Perform search with filters
    let results = if search_query.is_empty() {
        Vec::new()
    } else {
        search_engine.search_with_filters(index, &search_query, RESULT_LIMIT, &filters)
    };

    Ok(results)
}

/// Extract search query without filter syntax
fn extract_search_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|part| !part.contains(':'))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Display header
fn display_header(stdout: &mut std::io::Stdout, show_help: bool) -> Result<()> {
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print("Nothing - Interactive Search (Real-Time Monitoring)\n"),
        ResetColor,
        Print("📡 Index auto-updates as files change\n"),
        SetForegroundColor(Color::DarkGrey),
        Print("Press F1 for help • F2 for stats • Ctrl+C to exit\n"),
        ResetColor,
        Print("\n> "),
    )?;

    stdout.flush()?;
    Ok(())
}

/// Display search results
fn display_search_results(
    stdout: &mut std::io::Stdout,
    results: &[crate::search::SearchResult],
    query: &str,
    search_time: std::time::Duration,
    show_help: bool,
    show_stats: bool,
    metrics: &SearchMetrics,
) -> Result<()> {
    // Clear screen
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    // Header
    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print("Nothing - Interactive Search"),
        ResetColor,
    )?;

    // Parse and display active filters
    let filters = SearchFilters::parse_filter_string(query).unwrap_or_default();
    if !filters.is_empty() {
        execute!(
            stdout,
            SetForegroundColor(Color::Yellow),
            Print(format!(" [Filters: {}]", filters.describe())),
            ResetColor,
        )?;
    }

    execute!(
        stdout,
        Print("\n"),
        SetForegroundColor(Color::DarkGrey),
        Print(format!("Press F1 for help • F2 for stats • Ctrl+E to export • Ctrl+C to exit ({}ms)\n",
            search_time.as_millis())),
        ResetColor,
        Print("\n"),
    )?;

    // Show help if toggled
    if show_help {
        display_help(stdout)?;
        execute!(stdout, Print("\n"))?;
    }

    // Show stats if toggled
    if show_stats {
        display_stats(stdout, metrics)?;
        execute!(stdout, Print("\n"))?;
    }

    // Show query
    let search_query = extract_search_query(query);
    execute!(
        stdout,
        Print("> "),
        SetForegroundColor(Color::Yellow),
        Print(&search_query),
        ResetColor,
    )?;

    // Show filter parts in different color
    let filter_parts: Vec<&str> = query.split_whitespace()
        .filter(|p| p.contains(':'))
        .collect();
    if !filter_parts.is_empty() {
        execute!(
            stdout,
            Print(" "),
            SetForegroundColor(Color::Cyan),
            Print(filter_parts.join(" ")),
            ResetColor,
        )?;
    }

    execute!(stdout, Print("\n\n"))?;

    if search_query.is_empty() {
        stdout.flush()?;
        return Ok(());
    }

    // Count total matches (including those beyond limit)
    let total_matches = results.len(); // This is limited to RESULT_LIMIT in the search

    // Show match count
    if total_matches == 0 {
        execute!(
            stdout,
            SetForegroundColor(Color::Red),
            Print("No matches found\n"),
            ResetColor,
        )?;
    } else {
        execute!(
            stdout,
            SetForegroundColor(Color::Green),
            Print(format!(
                "Found matches (showing top {})\n\n",
                RESULT_LIMIT.min(total_matches)
            )),
            ResetColor,
        )?;

        // Display results
        for (idx, result) in results.iter().enumerate() {
            let entry_type = if result.entry.is_directory {
                "[DIR]"
            } else {
                "[FILE]"
            };

            // Format size
            let size_str = if result.entry.is_directory {
                String::new()
            } else {
                format!(" - {}", format_file_size(result.entry.size))
            };

            // Format modified date
            let modified_str = if let Some(modified) = result.entry.modified {
                format!(" - {}", modified.format("%Y-%m-%d %H:%M"))
            } else {
                String::new()
            };

            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{:2}. ", idx + 1)),
                ResetColor,
                SetForegroundColor(if result.entry.is_directory {
                    Color::Blue
                } else {
                    Color::White
                }),
                Print(format!("{} ", entry_type)),
                ResetColor,
                SetForegroundColor(Color::Green),
                Print(&result.entry.name),
                ResetColor,
                SetForegroundColor(Color::Yellow),
                Print(&size_str),
                ResetColor,
                SetForegroundColor(Color::Cyan),
                Print(&modified_str),
                ResetColor,
                Print("\n    "),
                SetForegroundColor(Color::DarkGrey),
                Print(&result.entry.path),
                ResetColor,
                Print(format!(" (score: {})\n", result.score)),
            )?;
        }
    }

    stdout.flush()?;
    Ok(())
}

/// Display help text
fn display_help(stdout: &mut std::io::Stdout) -> Result<()> {
    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print("╔═══════════════════════════════════════════════════════════════════╗\n"),
        Print("║                        SEARCH HELP                                ║\n"),
        Print("╠═══════════════════════════════════════════════════════════════════╣\n"),
        ResetColor,
        SetForegroundColor(Color::Yellow),
        Print("║ Filters:                                                          ║\n"),
        ResetColor,
        Print("║   size:>100mb        Files larger than 100MB                     ║\n"),
        Print("║   size:<1gb          Files smaller than 1GB                      ║\n"),
        Print("║   size:100kb-500kb   Files between 100KB and 500KB               ║\n"),
        Print("║   ext:rs,md          Files with .rs or .md extensions            ║\n"),
        Print("║   modified:7d        Modified in last 7 days                     ║\n"),
        Print("║   modified:>2024-01-01  Modified after Jan 1, 2024               ║\n"),
        Print("║   type:file          Files only (use type:dir for dirs)          ║\n"),
        SetForegroundColor(Color::Yellow),
        Print("║ Keyboard Shortcuts:                                               ║\n"),
        ResetColor,
        Print("║   ↑/↓                Navigate search history                     ║\n"),
        Print("║   Ctrl+E             Export results to CSV/JSON                  ║\n"),
        Print("║   F1                 Toggle this help                            ║\n"),
        Print("║   F2                 Toggle statistics                           ║\n"),
        Print("║   Ctrl+C             Exit                                        ║\n"),
        SetForegroundColor(Color::Yellow),
        Print("║ Examples:                                                         ║\n"),
        ResetColor,
        Print("║   readme ext:md      Find README files with .md extension        ║\n"),
        Print("║   video size:>100mb  Find videos larger than 100MB               ║\n"),
        Print("║   config modified:7d Recent config files                         ║\n"),
        SetForegroundColor(Color::Cyan),
        Print("╚═══════════════════════════════════════════════════════════════════╝\n"),
        ResetColor,
    )?;
    Ok(())
}

/// Display statistics
fn display_stats(stdout: &mut std::io::Stdout, metrics: &SearchMetrics) -> Result<()> {
    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print("╔═══════════════════════════════════════════════════════════════════╗\n"),
        Print("║                      SEARCH STATISTICS                            ║\n"),
        Print("╠═══════════════════════════════════════════════════════════════════╣\n"),
        ResetColor,
    )?;

    if metrics.total_searches == 0 {
        execute!(
            stdout,
            Print("║ No searches performed yet                                        ║\n"),
            SetForegroundColor(Color::Cyan),
            Print("╚═══════════════════════════════════════════════════════════════════╝\n"),
            ResetColor,
        )?;
    } else {
        execute!(
            stdout,
            Print(format!("║ Total searches:       {:>42} ║\n", metrics.total_searches)),
            Print(format!("║ Searches with results: {:>40.1}% ║\n", metrics.success_rate() * 100.0)),
            Print(format!("║ Total matches:        {:>42} ║\n", metrics.total_matches)),
            Print(format!("║ Average matches:      {:>42.1} ║\n", metrics.average_matches())),
            Print(format!("║ Average search time:  {:>39.2}ms ║\n", metrics.average_search_time().as_secs_f64() * 1000.0)),
            Print(format!("║ Fastest search:       {:>39.2}ms ║\n", metrics.fastest_search.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0))),
            Print(format!("║ Slowest search:       {:>39.2}ms ║\n", metrics.slowest_search.map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0))),
            SetForegroundColor(Color::Cyan),
            Print("╚═══════════════════════════════════════════════════════════════════╝\n"),
            ResetColor,
        )?;
    }

    Ok(())
}

/// Export results with user prompt
fn export_results(stdout: &mut std::io::Stdout, results: &[crate::search::SearchResult]) -> Result<()> {
    // Clear screen and show export prompt
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        SetForegroundColor(Color::Cyan),
        Print("Export Results\n\n"),
        ResetColor,
        Print(format!("Found {} results to export\n\n", results.len())),
        Print("Choose format:\n"),
        Print("  1. CSV\n"),
        Print("  2. JSON\n"),
        Print("  ESC to cancel\n\n"),
        Print("Choice: "),
    )?;
    stdout.flush()?;

    // Read choice
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('1') => {
                        let path = format!("search_results_{}.csv", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                        export::export_csv(results, &path)?;
                        execute!(
                            stdout,
                            Print("\n\n"),
                            SetForegroundColor(Color::Green),
                            Print(format!("✓ Exported to {}\n", path)),
                            ResetColor,
                            Print("\nPress any key to continue..."),
                        )?;
                        stdout.flush()?;
                        event::read()?;
                        break;
                    }
                    KeyCode::Char('2') => {
                        let path = format!("search_results_{}.json", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                        export::export_json(results, &path)?;
                        execute!(
                            stdout,
                            Print("\n\n"),
                            SetForegroundColor(Color::Green),
                            Print(format!("✓ Exported to {}\n", path)),
                            ResetColor,
                            Print("\nPress any key to continue..."),
                        )?;
                        stdout.flush()?;
                        event::read()?;
                        break;
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}
