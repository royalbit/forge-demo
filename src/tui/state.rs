//! TUI state types - enums for input mode, filter mode, and active panel.

use ratatui::style::Color;

// ─────────────────────────────────────────────────────────────────────────────
// Input Mode
// ─────────────────────────────────────────────────────────────────────────────

/// The current input mode for the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Normal navigation mode.
    #[default]
    Normal,
    /// Search mode - typing filters results.
    Search,
}

// ─────────────────────────────────────────────────────────────────────────────
// Category Colors
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the color for a test category based on its name prefix.
pub fn category_color(name: &str) -> Color {
    let category = name.split('.').next().unwrap_or("");
    match category {
        "math" | "aggregation" => Color::Blue,
        "text" => Color::Yellow,
        "date" => Color::Magenta,
        "logical" => Color::Cyan,
        "lookup" => Color::Green,
        _ => Color::White,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Filter Mode
// ─────────────────────────────────────────────────────────────────────────────

/// Filter mode for the results list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterMode {
    /// Show all test results.
    #[default]
    All,
    /// Show only passed tests.
    Passed,
    /// Show only failed tests.
    Failed,
}

impl FilterMode {
    /// Returns the display label for this filter mode.
    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Passed => "Passed",
            Self::Failed => "Failed",
        }
    }

    /// Returns the keyboard shortcut for this filter mode.
    pub const fn shortcut(self) -> char {
        match self {
            Self::All => '1',
            Self::Passed => '2',
            Self::Failed => '3',
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Active Panel
// ─────────────────────────────────────────────────────────────────────────────

/// The currently active panel in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePanel {
    /// The results list panel.
    #[default]
    Results,
    /// The detail pane showing selected test info.
    Details,
    /// The statistics panel.
    Stats,
}

impl ActivePanel {
    /// Cycle to the next panel.
    pub const fn next(self) -> Self {
        match self {
            Self::Results => Self::Details,
            Self::Details => Self::Stats,
            Self::Stats => Self::Results,
        }
    }

    /// Cycle to the previous panel.
    pub const fn prev(self) -> Self {
        match self {
            Self::Results => Self::Stats,
            Self::Details => Self::Results,
            Self::Stats => Self::Details,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_mode_labels() {
        assert_eq!(FilterMode::All.label(), "All");
        assert_eq!(FilterMode::Passed.label(), "Passed");
        assert_eq!(FilterMode::Failed.label(), "Failed");
    }

    #[test]
    fn filter_mode_shortcuts() {
        assert_eq!(FilterMode::All.shortcut(), '1');
        assert_eq!(FilterMode::Passed.shortcut(), '2');
        assert_eq!(FilterMode::Failed.shortcut(), '3');
    }

    #[test]
    fn filter_mode_default() {
        assert_eq!(FilterMode::default(), FilterMode::All);
    }

    #[test]
    fn active_panel_next_cycles() {
        assert_eq!(ActivePanel::Results.next(), ActivePanel::Details);
        assert_eq!(ActivePanel::Details.next(), ActivePanel::Stats);
        assert_eq!(ActivePanel::Stats.next(), ActivePanel::Results);
    }

    #[test]
    fn active_panel_prev_cycles() {
        assert_eq!(ActivePanel::Results.prev(), ActivePanel::Stats);
        assert_eq!(ActivePanel::Details.prev(), ActivePanel::Results);
        assert_eq!(ActivePanel::Stats.prev(), ActivePanel::Details);
    }

    #[test]
    fn active_panel_default() {
        assert_eq!(ActivePanel::default(), ActivePanel::Results);
    }

    #[test]
    fn input_mode_default() {
        assert_eq!(InputMode::default(), InputMode::Normal);
    }

    #[test]
    fn category_colors() {
        assert_eq!(category_color("math.ABS"), Color::Blue);
        assert_eq!(category_color("aggregation.SUM"), Color::Blue);
        assert_eq!(category_color("text.CONCAT"), Color::Yellow);
        assert_eq!(category_color("date.TODAY"), Color::Magenta);
        assert_eq!(category_color("logical.IF"), Color::Cyan);
        assert_eq!(category_color("lookup.CHOOSE"), Color::Green);
        assert_eq!(category_color("unknown.TEST"), Color::White);
    }
}
