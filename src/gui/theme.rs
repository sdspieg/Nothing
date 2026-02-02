use iced::Theme;
use serde::{Deserialize, Serialize};

/// Application theme preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppTheme {
    Dark,
    Light,
}

impl AppTheme {
    /// Toggle between dark and light themes
    pub fn toggle(&mut self) {
        *self = match self {
            AppTheme::Dark => AppTheme::Light,
            AppTheme::Light => AppTheme::Dark,
        };
    }

    /// Get the iced Theme for this app theme
    pub fn to_iced_theme(&self) -> Theme {
        match self {
            AppTheme::Dark => Theme::TokyoNightStorm,
            AppTheme::Light => Theme::CatppuccinLatte,
        }
    }

    /// Get theme name for display
    pub fn name(&self) -> &str {
        match self {
            AppTheme::Dark => "Dark",
            AppTheme::Light => "Light",
        }
    }

    /// Get icon for theme toggle button
    pub fn icon(&self) -> &str {
        match self {
            AppTheme::Dark => "☀️",  // Sun icon for switching to light
            AppTheme::Light => "🌙", // Moon icon for switching to dark
        }
    }
}

impl Default for AppTheme {
    fn default() -> Self {
        AppTheme::Dark
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_toggle() {
        let mut theme = AppTheme::Dark;
        theme.toggle();
        assert_eq!(theme, AppTheme::Light);
        theme.toggle();
        assert_eq!(theme, AppTheme::Dark);
    }
}
