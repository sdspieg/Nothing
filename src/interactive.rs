use crate::index::FileIndex;
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

    // Enable raw mode for character-by-character input
    terminal::enable_raw_mode()?;

    // Clear screen and show initial prompt
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    println!("Nothing - Interactive Search (Real-Time Monitoring)");
    println!("📡 Index auto-updates as files change");
    println!("Press Ctrl+C or Ctrl+D to exit");
    println!("Type to search (fuzzy matching enabled)...\n");
    print!("> ");
    stdout.flush()?;

    loop {
        // Read keyboard events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                match handle_key_event(key_event, &mut query)? {
                    KeyAction::Exit => break,
                    KeyAction::UpdateSearch => {
                        // Lock index for searching
                        let index_guard = index.lock().unwrap();
                        display_search_results(&mut stdout, &mut search_engine, &*index_guard, &query)?;
                        drop(index_guard); // Release lock
                    }
                    KeyAction::None => {}
                }
            }
        }
    }

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

    // Enable raw mode for character-by-character input
    terminal::enable_raw_mode()?;

    // Clear screen and show initial prompt
    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    println!("Nothing - Interactive Search");
    println!("Press Ctrl+C or Ctrl+D to exit");
    println!("Type to search (fuzzy matching enabled)...\n");
    print!("> ");
    stdout.flush()?;

    loop {
        // Read keyboard events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                match handle_key_event(key_event, &mut query)? {
                    KeyAction::Exit => break,
                    KeyAction::UpdateSearch => {
                        display_search_results(&mut stdout, &mut search_engine, index, &query)?;
                    }
                    KeyAction::None => {}
                }
            }
        }
    }

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
    None,
}

/// Handle keyboard input
fn handle_key_event(key: KeyEvent, query: &mut String) -> Result<KeyAction> {
    match key.code {
        // Exit on Ctrl+C or Ctrl+D
        KeyCode::Char('c') | KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Ok(KeyAction::Exit)
        }

        // Backspace
        KeyCode::Backspace => {
            query.pop();
            Ok(KeyAction::UpdateSearch)
        }

        // Regular character input
        KeyCode::Char(c) => {
            query.push(c);
            Ok(KeyAction::UpdateSearch)
        }

        // Enter - just update display
        KeyCode::Enter => Ok(KeyAction::UpdateSearch),

        // Ignore other keys
        _ => Ok(KeyAction::None),
    }
}

/// Display search results
fn display_search_results(
    stdout: &mut std::io::Stdout,
    search_engine: &mut SearchEngine,
    index: &FileIndex,
    query: &str,
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
        Print("Nothing - Interactive Search\n"),
        ResetColor,
        Print("Press Ctrl+C or Ctrl+D to exit\n"),
        Print("Type to search (fuzzy matching enabled)...\n\n"),
    )?;

    // Show query
    execute!(
        stdout,
        Print("> "),
        SetForegroundColor(Color::Yellow),
        Print(query),
        ResetColor,
        Print("\n\n"),
    )?;

    if query.is_empty() {
        stdout.flush()?;
        return Ok(());
    }

    // Perform search
    let results = search_engine.search(index, query, RESULT_LIMIT);
    let total_matches = search_engine.count_matches(index, query);

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
                "Found {} match{} (showing top {})\n\n",
                total_matches,
                if total_matches == 1 { "" } else { "es" },
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
