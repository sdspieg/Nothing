use crate::filters::SearchFilters;
use crate::gui::theme::AppTheme;
use crate::index::FileIndex;
use crate::search::{SearchEngine, SearchResult};
use iced::{
    widget::{button, column, container, row, scrollable, text, text_input, Column, Space},
    Alignment, Element, Length, Task, Theme,
};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Main application state
pub struct NothingGui {
    /// File index (shared with monitoring threads)
    index: Arc<Mutex<FileIndex>>,

    /// Search engine
    search_engine: SearchEngine,

    /// Current search query
    query: String,

    /// Current search results
    results: Vec<SearchResult>,

    /// Current theme
    theme: AppTheme,

    /// Whether filter panel is visible
    show_filters: bool,

    /// Whether stats panel is visible
    show_stats: bool,

    /// Current filters
    filters: SearchFilters,

    /// Search timing
    last_search_time: Option<std::time::Duration>,

    /// Sort column and direction
    sort_by: SortColumn,
    sort_ascending: bool,

    /// Selected result index
    selected_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Path,
    Size,
    Modified,
    Type,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    /// Search query changed
    SearchChanged(String),

    /// Perform the actual search (debounced)
    PerformSearch,

    /// Theme toggled
    ToggleTheme,

    /// Filter panel toggled
    ToggleFilters,

    /// Stats panel toggled
    ToggleStats,

    /// Sort column changed
    SortBy(SortColumn),

    /// Result selected
    SelectResult(usize),

    /// Open selected result
    OpenResult,

    /// Open containing folder
    OpenFolder,

    /// Export results to CSV
    ExportCSV,

    /// Export results to JSON
    ExportJSON,

    /// Copy path to clipboard
    CopyPath,

    /// Show settings
    ShowSettings,

    /// Filter changed
    FilterChanged(SearchFilters),

    /// Clear filters
    ClearFilters,

    /// Clear search query
    ClearSearch,

    /// Navigate results (up/down)
    NavigateResults(i32), // -1 for up, +1 for down

    /// Keyboard event
    KeyPressed(iced::keyboard::Key, iced::keyboard::Modifiers),
}

impl NothingGui {
    fn new(index: Arc<Mutex<FileIndex>>) -> Self {
        Self {
            index,
            search_engine: SearchEngine::new(),
            query: String::new(),
            results: Vec::new(),
            theme: AppTheme::default(),
            show_filters: true,
            show_stats: false,
            filters: SearchFilters::default(),
            last_search_time: None,
            sort_by: SortColumn::Name,
            sort_ascending: true,
            selected_index: None,
        }
    }

    fn title(&self) -> String {
        format!("Nothing - Fast File Search ({})", self.theme.name())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(query) => {
                self.query = query;
                // Debounce: only perform search after 150ms delay
                return Task::perform(
                    async {
                        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                    },
                    |_| Message::PerformSearch,
                );
            }

            Message::PerformSearch => {
                self.perform_search();
            }

            Message::ToggleTheme => {
                self.theme.toggle();
            }

            Message::ToggleFilters => {
                self.show_filters = !self.show_filters;
            }

            Message::ToggleStats => {
                self.show_stats = !self.show_stats;
            }

            Message::SortBy(column) => {
                if self.sort_by == column {
                    self.sort_ascending = !self.sort_ascending;
                } else {
                    self.sort_by = column;
                    self.sort_ascending = true;
                }
                self.sort_results();
            }

            Message::SelectResult(index) => {
                self.selected_index = Some(index);
            }

            Message::OpenResult => {
                if let Some(index) = self.selected_index {
                    if let Some(result) = self.results.get(index) {
                        let _ = open::that(&result.entry.path);
                    }
                }
            }

            Message::OpenFolder => {
                if let Some(index) = self.selected_index {
                    if let Some(result) = self.results.get(index) {
                        let path = std::path::Path::new(&result.entry.path);
                        if let Some(parent) = path.parent() {
                            let _ = open::that(parent);
                        }
                    }
                }
            }

            Message::ExportCSV => {
                if !self.results.is_empty() {
                    // Use file dialog to select save location
                    let file_dialog = rfd::FileDialog::new()
                        .add_filter("CSV", &["csv"])
                        .set_file_name("search_results.csv");

                    if let Some(path) = file_dialog.save_file() {
                        use crate::export;
                        if let Err(e) = export::export_csv(&self.results, path.to_str().unwrap()) {
                            eprintln!("Export failed: {}", e);
                        }
                    }
                }
            }

            Message::ExportJSON => {
                if !self.results.is_empty() {
                    let file_dialog = rfd::FileDialog::new()
                        .add_filter("JSON", &["json"])
                        .set_file_name("search_results.json");

                    if let Some(path) = file_dialog.save_file() {
                        use crate::export;
                        if let Err(e) = export::export_json(&self.results, path.to_str().unwrap()) {
                            eprintln!("Export failed: {}", e);
                        }
                    }
                }
            }

            Message::CopyPath => {
                if let Some(index) = self.selected_index {
                    if let Some(result) = self.results.get(index) {
                        // Use iced's clipboard (if available) or system clipboard
                        use std::process::Command;
                        let _ = Command::new("cmd")
                            .args(&["/C", "echo", &result.entry.path, "|", "clip"])
                            .output();
                    }
                }
            }

            Message::ShowSettings => {
                // TODO: Show settings dialog
            }

            Message::FilterChanged(filters) => {
                self.filters = filters;
                self.perform_search();
            }

            Message::ClearFilters => {
                self.filters = SearchFilters::default();
                self.perform_search();
            }

            Message::ClearSearch => {
                self.query.clear();
                self.results.clear();
                self.selected_index = None;
            }

            Message::NavigateResults(delta) => {
                if !self.results.is_empty() {
                    if let Some(current) = self.selected_index {
                        let new_index = (current as i32 + delta)
                            .max(0)
                            .min(self.results.len() as i32 - 1) as usize;
                        self.selected_index = Some(new_index);
                    } else if delta > 0 {
                        self.selected_index = Some(0);
                    } else {
                        self.selected_index = Some(self.results.len() - 1);
                    }
                }
            }

            Message::KeyPressed(key, modifiers) => {
                use iced::keyboard::{Key, key::Named};

                match key.as_ref() {
                    Key::Named(Named::Escape) => {
                        self.query.clear();
                        self.results.clear();
                        self.selected_index = None;
                    }
                    Key::Named(Named::Enter) => {
                        if let Some(index) = self.selected_index {
                            if let Some(result) = self.results.get(index) {
                                let _ = open::that(&result.entry.path);
                            }
                        }
                    }
                    Key::Named(Named::ArrowDown) => {
                        if !self.results.is_empty() {
                            if let Some(current) = self.selected_index {
                                self.selected_index = Some((current + 1).min(self.results.len() - 1));
                            } else {
                                self.selected_index = Some(0);
                            }
                        }
                    }
                    Key::Named(Named::ArrowUp) => {
                        if !self.results.is_empty() {
                            if let Some(current) = self.selected_index {
                                self.selected_index = Some(current.saturating_sub(1));
                            }
                        }
                    }
                    Key::Character(c) if c == "o" && modifiers.control() => {
                        if let Some(index) = self.selected_index {
                            if let Some(result) = self.results.get(index) {
                                let path = std::path::Path::new(&result.entry.path);
                                if let Some(parent) = path.parent() {
                                    let _ = open::that(parent);
                                }
                            }
                        }
                    }
                    Key::Character(c) if c == "c" && modifiers.control() => {
                        // Copy path to clipboard
                        if let Some(index) = self.selected_index {
                            if let Some(result) = self.results.get(index) {
                                use std::process::Command;
                                let _ = Command::new("cmd")
                                    .args(&["/C", "echo", &result.entry.path, "|", "clip"])
                                    .output();
                            }
                        }
                    }
                    Key::Character(c) if c == "e" && modifiers.control() => {
                        // Export to CSV
                        if !self.results.is_empty() {
                            return Task::perform(async {}, |_| Message::ExportCSV);
                        }
                    }
                    _ => {}
                }
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<Message> {
        let title_bar = self.view_title_bar();
        let search_bar = self.view_search_bar();
        let content = self.view_content();
        let status_bar = self.view_status_bar();

        let main_column = column![title_bar, search_bar, content, status_bar]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        container(main_column)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn theme(&self) -> Theme {
        self.theme.to_iced_theme()
    }
}

impl NothingGui {
    /// Perform search with current query and filters
    fn perform_search(&mut self) {
        if self.query.is_empty() {
            self.results.clear();
            self.last_search_time = None;
            return;
        }

        let start = Instant::now();
        let index = self.index.lock().unwrap();

        self.results = self.search_engine.search_with_filters(
            &index,
            &self.query,
            100, // Show top 100 results in GUI
            &self.filters,
        );

        drop(index);
        self.last_search_time = Some(start.elapsed());
        self.sort_results();
    }

    /// Sort results based on current sort column
    fn sort_results(&mut self) {
        let ascending = self.sort_ascending;

        self.results.sort_by(|a, b| {
            let cmp = match self.sort_by {
                SortColumn::Name => a.entry.name.cmp(&b.entry.name),
                SortColumn::Path => a.entry.path.cmp(&b.entry.path),
                SortColumn::Size => a.entry.size.cmp(&b.entry.size),
                SortColumn::Modified => a
                    .entry
                    .modified
                    .unwrap_or_default()
                    .cmp(&b.entry.modified.unwrap_or_default()),
                SortColumn::Type => a.entry.is_directory.cmp(&b.entry.is_directory),
            };

            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
    }

    /// View title bar with menu and theme toggle
    fn view_title_bar(&self) -> Element<Message> {
        let theme_button = button(text(self.theme.icon()).size(20))
            .on_press(Message::ToggleTheme)
            .padding(8);

        let filters_button = button(
            text(if self.show_filters {
                "Hide Filters"
            } else {
                "Show Filters"
            })
            .size(14),
        )
        .on_press(Message::ToggleFilters)
        .padding(8);

        let stats_button = button(
            text(if self.show_stats {
                "Hide Stats"
            } else {
                "Show Stats"
            })
            .size(14),
        )
        .on_press(Message::ToggleStats)
        .padding(8);

        let export_csv_button = button(text("📄 Export CSV").size(14))
            .on_press(Message::ExportCSV)
            .padding(8);

        let export_json_button = button(text("📋 Export JSON").size(14))
            .on_press(Message::ExportJSON)
            .padding(8);

        let title_row = row![
            text("Nothing - Fast File Search")
                .size(16)
                .width(Length::Fill),
            export_csv_button,
            export_json_button,
            filters_button,
            stats_button,
            theme_button,
        ]
        .spacing(10)
        .padding(10)
        .align_y(Alignment::Center);

        container(title_row)
            .width(Length::Fill)
            .style(|theme: &Theme| {
                container::Style::default()
                    .border(iced::Border {
                        width: 0.0,
                        color: theme.palette().background,
                        radius: 0.0.into(),
                    })
                    .background(theme.palette().primary)
            })
            .into()
    }

    /// View search bar
    fn view_search_bar(&self) -> Element<Message> {
        let search_input = text_input("Search files...", &self.query)
            .on_input(Message::SearchChanged)
            .padding(15)
            .size(18)
            .width(Length::Fill);

        let hints = text("↑/↓ Navigate • Enter Open • Esc Clear • Ctrl+O Open Folder • Ctrl+C Copy Path • Ctrl+E Export")
            .size(11)
            .style(|theme: &Theme| text::Style {
                color: Some(theme.extended_palette().background.strong.text),
            });

        let search_col = column![search_input, hints]
            .spacing(5)
            .width(Length::Fill);

        container(search_col)
            .padding(20)
            .width(Length::Fill)
            .into()
    }

    /// View main content (filter panel + results)
    fn view_content(&self) -> Element<Message> {
        let results_view = self.view_results();

        if self.show_filters {
            let filter_panel = self.view_filter_panel();

            row![filter_panel, results_view]
                .spacing(10)
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(results_view)
                .padding(10)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
    }

    /// View filter panel
    fn view_filter_panel(&self) -> Element<Message> {
        let title = text("Filters").size(16).width(Length::Fill);

        let clear_button = button(text("Clear All").size(14))
            .on_press(Message::ClearFilters)
            .padding(8)
            .width(Length::Fill);

        let filter_column = column![title, Space::with_height(20), clear_button]
            .spacing(10)
            .padding(15)
            .width(Length::Fixed(250.0));

        container(filter_column)
            .width(Length::Fixed(250.0))
            .height(Length::Fill)
            .style(|theme: &Theme| {
                container::Style::default()
                    .border(iced::Border {
                        width: 1.0,
                        color: theme.palette().background,
                        radius: 8.0.into(),
                    })
                    .background(theme.extended_palette().background.weak.color)
            })
            .into()
    }

    /// View results table
    fn view_results(&self) -> Element<Message> {
        if self.query.is_empty() {
            let empty_message = column![
                Space::with_height(Length::Fill),
                text("Type to search...")
                    .size(20)
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center),
                Space::with_height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill);

            return container(empty_message)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        if self.results.is_empty() {
            let no_results = column![
                Space::with_height(Length::Fill),
                text("No matches found")
                    .size(20)
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center),
                Space::with_height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill);

            return container(no_results)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        }

        // Table header
        let header = row![
            button(text("Name").size(14))
                .on_press(Message::SortBy(SortColumn::Name))
                .padding(8)
                .width(Length::FillPortion(3)),
            button(text("Path").size(14))
                .on_press(Message::SortBy(SortColumn::Path))
                .padding(8)
                .width(Length::FillPortion(5)),
            button(text("Size").size(14))
                .on_press(Message::SortBy(SortColumn::Size))
                .padding(8)
                .width(Length::FillPortion(2)),
            button(text("Modified").size(14))
                .on_press(Message::SortBy(SortColumn::Modified))
                .padding(8)
                .width(Length::FillPortion(2)),
        ]
        .spacing(5)
        .padding(10);

        // Table rows
        let mut rows = Column::new().spacing(2).padding(5);

        for (index, result) in self.results.iter().enumerate() {
            let entry_type = if result.entry.is_directory {
                "📁"
            } else {
                "📄"
            };

            let size_str = if result.entry.is_directory {
                String::new()
            } else {
                format_file_size(result.entry.size)
            };

            let modified_str = result
                .entry
                .modified
                .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();

            let is_selected = self.selected_index == Some(index);

            let row_content = row![
                text(format!("{} {}", entry_type, result.entry.name))
                    .size(14)
                    .width(Length::FillPortion(3)),
                text(&result.entry.path)
                    .size(12)
                    .width(Length::FillPortion(5)),
                text(size_str).size(12).width(Length::FillPortion(2)),
                text(modified_str).size(12).width(Length::FillPortion(2)),
            ]
            .spacing(5)
            .padding(8);

            let row_button = button(row_content)
                .on_press(Message::SelectResult(index))
                .width(Length::Fill)
                .style(move |theme: &Theme, status| {
                    let mut style = button::Style::default();
                    if is_selected {
                        style.background = Some(theme.palette().primary.into());
                    } else if matches!(status, button::Status::Hovered) {
                        style.background = Some(theme.extended_palette().background.weak.color.into());
                    }
                    style
                });

            rows = rows.push(row_button);
        }

        let table = column![header, scrollable(rows).height(Length::Fill)]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill);

        container(table)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|theme: &Theme| {
                container::Style::default()
                    .border(iced::Border {
                        width: 1.0,
                        color: theme.palette().background,
                        radius: 8.0.into(),
                    })
            })
            .into()
    }

    /// View status bar
    fn view_status_bar(&self) -> Element<Message> {
        let index = self.index.lock().unwrap();
        let file_count = index.file_count();
        let dir_count = index.directory_count();
        drop(index);

        let search_time = self
            .last_search_time
            .map(|d| format!("{}ms", d.as_millis()))
            .unwrap_or_else(|| "—".to_string());

        let status_text = format!(
            "📊 {} files, {} directories • Results: {} • Search: {}",
            file_count,
            dir_count,
            self.results.len(),
            search_time
        );

        let status_row = row![text(status_text).size(12),]
            .spacing(10)
            .padding(10)
            .align_y(Alignment::Center);

        container(status_row)
            .width(Length::Fill)
            .style(|theme: &Theme| {
                container::Style::default()
                    .background(theme.extended_palette().background.weak.color)
            })
            .into()
    }

    /// Subscribe to keyboard events
    fn subscription(&self) -> iced::Subscription<Message> {
        use iced::keyboard;
        use iced::event;

        event::listen_with(|event, _status, _window| {
            if let event::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                Some(Message::KeyPressed(key, modifiers))
            } else {
                None
            }
        })
    }
}

/// Format file size
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

/// Run the GUI application
pub fn run(index: Arc<Mutex<FileIndex>>) -> iced::Result {
    iced::application(
        NothingGui::title,
        NothingGui::update,
        NothingGui::view,
    )
    .subscription(NothingGui::subscription)
    .theme(NothingGui::theme)
    .window(iced::window::Settings {
        size: iced::Size::new(1200.0, 800.0),
        position: iced::window::Position::Centered,
        min_size: Some(iced::Size::new(800.0, 600.0)),
        ..Default::default()
    })
    .run_with(move || {
        let app = NothingGui::new(index.clone());
        (app, Task::none())
    })
}
