//! TUI interface using ratatui
//!
//! Provides an interactive terminal UI for running E2E validation tests.
//! Features:
//! - Scrollable results list with keyboard navigation (j/k, arrows)
//! - Tab between panels (results, details, stats)
//! - Detail pane showing formula, expected, and actual values
//! - Filter view: all/passed/failed (toggle with 1/2/3 keys)
//! - Search mode with `/` key to filter by test name
//! - Color-coded function categories (math, text, date, etc.)

use std::fmt::Write as _;
use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::runner::TestRunner;
use crate::types::TestResult;

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
fn category_color(name: &str) -> Color {
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
    const fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Passed => "Passed",
            Self::Failed => "Failed",
        }
    }

    /// Returns the keyboard shortcut for this filter mode.
    const fn shortcut(self) -> char {
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
    const fn next(self) -> Self {
        match self {
            Self::Results => Self::Details,
            Self::Details => Self::Stats,
            Self::Stats => Self::Results,
        }
    }

    /// Cycle to the previous panel.
    const fn prev(self) -> Self {
        match self {
            Self::Results => Self::Stats,
            Self::Details => Self::Results,
            Self::Stats => Self::Details,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// App State
// ─────────────────────────────────────────────────────────────────────────────

/// Main application state for the TUI.
pub struct App {
    /// All test results collected so far.
    results: Vec<TestResult>,
    /// Index of the currently executing test.
    current_test: usize,
    /// Total number of tests to run.
    total_tests: usize,
    /// Whether tests are still running.
    running: bool,
    /// Whether all tests have completed.
    done: bool,
    /// Count of passed tests.
    passed: usize,
    /// Count of failed tests.
    failed: usize,
    /// Count of skipped tests.
    skipped: usize,
    /// Currently active panel.
    active_panel: ActivePanel,
    /// Current filter mode.
    filter_mode: FilterMode,
    /// State for the results list (selection, scroll offset).
    list_state: ListState,
    /// Cached filtered indices for the current filter mode.
    filtered_indices: Vec<usize>,
    /// Current input mode (normal or search).
    input_mode: InputMode,
    /// Search query string.
    search_query: String,
}

impl App {
    /// Creates a new [`App`] with the given total test count.
    pub fn new(total: usize) -> Self {
        Self {
            results: Vec::with_capacity(total),
            current_test: 0,
            total_tests: total,
            running: true,
            done: false,
            passed: 0,
            failed: 0,
            skipped: 0,
            active_panel: ActivePanel::default(),
            filter_mode: FilterMode::default(),
            list_state: ListState::default(),
            filtered_indices: Vec::new(),
            input_mode: InputMode::default(),
            search_query: String::new(),
        }
    }

    /// Adds a test result and updates statistics.
    pub fn add_result(&mut self, result: TestResult) {
        match &result {
            TestResult::Pass { .. } => self.passed += 1,
            TestResult::Fail { .. } => self.failed += 1,
            TestResult::Skip { .. } => self.skipped += 1,
        }
        self.results.push(result);
        self.current_test += 1;
        self.update_filtered_indices();

        // Auto-select the latest result if nothing is selected
        if self.list_state.selected().is_none() && !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Marks the app as done running tests.
    pub const fn mark_done(&mut self) {
        self.running = false;
        self.done = true;
    }

    /// Updates the cached filtered indices based on the current filter mode and search query.
    fn update_filtered_indices(&mut self) {
        let query_lower = self.search_query.to_lowercase();
        self.filtered_indices = self
            .results
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                // Apply filter mode
                let passes_filter = match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::Passed => r.is_pass(),
                    FilterMode::Failed => r.is_fail(),
                };
                // Apply search query
                let passes_search =
                    query_lower.is_empty() || r.name().to_lowercase().contains(&query_lower);
                passes_filter && passes_search
            })
            .map(|(i, _)| i)
            .rev() // Most recent first
            .collect();
    }

    /// Enters search mode.
    pub const fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
    }

    /// Exits search mode and clears the query.
    pub fn exit_search_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.update_filtered_indices();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Appends a character to the search query.
    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filtered_indices();
        if self.filtered_indices.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    /// Removes the last character from the search query.
    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.update_filtered_indices();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Sets the filter mode and updates the filtered indices.
    pub fn set_filter(&mut self, mode: FilterMode) {
        if self.filter_mode != mode {
            self.filter_mode = mode;
            self.update_filtered_indices();
            // Reset selection to first item if current selection is invalid
            if self.filtered_indices.is_empty() {
                self.list_state.select(None);
            } else {
                self.list_state.select(Some(0));
            }
        }
    }

    /// Moves selection up in the results list.
    pub fn select_previous(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| i.saturating_sub(1));
        self.list_state.select(Some(i));
    }

    /// Moves selection down in the results list.
    pub fn select_next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let max_idx = self.filtered_indices.len().saturating_sub(1);
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| (i + 1).min(max_idx));
        self.list_state.select(Some(i));
    }

    /// Returns the currently selected test result, if any.
    pub fn selected_result(&self) -> Option<&TestResult> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&idx| self.results.get(idx))
    }

    /// Returns the filtered results for display.
    pub fn filtered_results(&self) -> Vec<&TestResult> {
        self.filtered_indices
            .iter()
            .filter_map(|&i| self.results.get(i))
            .collect()
    }

    /// Switches to the next panel.
    pub const fn next_panel(&mut self) {
        self.active_panel = self.active_panel.next();
    }

    /// Switches to the previous panel.
    pub const fn prev_panel(&mut self) {
        self.active_panel = self.active_panel.prev();
    }

    /// Returns the progress percentage (0-100).
    #[allow(clippy::cast_possible_truncation)]
    pub const fn progress_percent(&self) -> u16 {
        if self.total_tests > 0 {
            let percent = (self.current_test * 100) / self.total_tests;
            if percent > 100 {
                100
            } else {
                percent as u16
            }
        } else {
            0
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TUI Entry Point
// ─────────────────────────────────────────────────────────────────────────────

/// Runs the TUI interface.
///
/// Returns `Ok(true)` if all tests passed, `Ok(false)` otherwise.
pub fn run(runner: &TestRunner) -> anyhow::Result<bool> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run_app(&mut terminal, runner);

    // Restore terminal (always, even on error)
    let _ = disable_raw_mode();
    let _ = stdout().execute(LeaveAlternateScreen);

    result
}

/// Main application loop.
fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runner: &TestRunner,
) -> anyhow::Result<bool> {
    let total = runner.total_tests();
    let mut app = App::new(total);
    let test_cases = runner.test_cases().to_vec();

    // Run tests one by one
    for test_case in test_cases {
        // Check for quit or navigation keys
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(false);
                }
            }
        }

        // Draw current state
        terminal.draw(|frame| draw_ui(frame, &mut app))?;

        // Run test
        let result = runner.run_test(&test_case);
        app.add_result(result);

        // Draw updated state
        terminal.draw(|frame| draw_ui(frame, &mut app))?;
    }

    app.mark_done();

    // Final draw and interactive loop
    loop {
        terminal.draw(|frame| draw_ui(frame, &mut app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            // Exit
                            KeyCode::Char('q') | KeyCode::Esc => {
                                return Ok(app.failed == 0);
                            }
                            KeyCode::Enter if app.done => {
                                return Ok(app.failed == 0);
                            }
                            // Search mode
                            KeyCode::Char('/') => app.enter_search_mode(),
                            // Navigation
                            KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                            // Panel switching
                            KeyCode::Tab => app.next_panel(),
                            KeyCode::BackTab => app.prev_panel(),
                            // Filter modes
                            KeyCode::Char('1') => app.set_filter(FilterMode::All),
                            KeyCode::Char('2') => app.set_filter(FilterMode::Passed),
                            KeyCode::Char('3') => app.set_filter(FilterMode::Failed),
                            _ => {}
                        },
                        InputMode::Search => match key.code {
                            KeyCode::Esc => app.exit_search_mode(),
                            KeyCode::Enter => {
                                app.input_mode = InputMode::Normal;
                            }
                            KeyCode::Backspace => app.search_pop(),
                            KeyCode::Char(c) => app.search_push(c),
                            KeyCode::Up | KeyCode::Down => {
                                // Allow navigation while searching
                                if key.code == KeyCode::Up {
                                    app.select_previous();
                                } else {
                                    app.select_next();
                                }
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UI Drawing
// ─────────────────────────────────────────────────────────────────────────────

/// Draws the main UI layout.
fn draw_ui(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Main vertical layout
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Progress bar
            Constraint::Min(10),   // Main content area
            Constraint::Length(3), // Footer
        ])
        .split(area);

    draw_title(frame, main_chunks[0]);
    draw_progress(frame, main_chunks[1], app);

    // Split main content into left (results) and right (details + stats)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[2]);

    // Right side: details on top, stats on bottom
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(5)])
        .split(content_chunks[1]);

    draw_results_list(frame, content_chunks[0], app);
    draw_details(frame, right_chunks[0], app);
    draw_stats(frame, right_chunks[1], app);
    draw_footer(frame, main_chunks[3], app);
}

/// Draws the title bar.
fn draw_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("forge-e2e: E2E Validation Suite")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
}

/// Draws the progress gauge.
fn draw_progress(frame: &mut Frame, area: Rect, app: &App) {
    let progress = app.progress_percent();
    let label = format!("{}/{} tests", app.current_test, app.total_tests);

    let gauge = Gauge::default()
        .block(Block::default().title(" Progress ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(progress)
        .label(label);
    frame.render_widget(gauge, area);
}

/// Draws the scrollable results list.
fn draw_results_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_active = app.active_panel == ActivePanel::Results;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    // Build filter indicator
    let filter_label = format!(
        " Results [{}:{} {}:{} {}:{}] ",
        FilterMode::All.shortcut(),
        if app.filter_mode == FilterMode::All {
            FilterMode::All.label().to_uppercase()
        } else {
            FilterMode::All.label().to_string()
        },
        FilterMode::Passed.shortcut(),
        if app.filter_mode == FilterMode::Passed {
            FilterMode::Passed.label().to_uppercase()
        } else {
            FilterMode::Passed.label().to_string()
        },
        FilterMode::Failed.shortcut(),
        if app.filter_mode == FilterMode::Failed {
            FilterMode::Failed.label().to_uppercase()
        } else {
            FilterMode::Failed.label().to_string()
        },
    );

    let items: Vec<ListItem> = app
        .filtered_results()
        .iter()
        .map(|r| format_result_item(r))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(filter_label)
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

/// Formats a single test result as a list item with category coloring.
fn format_result_item(result: &TestResult) -> ListItem<'static> {
    let name = result.name();
    let cat_color = category_color(name);

    let (symbol, symbol_color, detail) = match result {
        TestResult::Pass { actual, .. } => ("✓", Color::Green, format!("= {actual}")),
        TestResult::Fail {
            expected,
            actual,
            error,
            ..
        } => {
            let err_detail = actual.map_or_else(
                || {
                    error
                        .as_ref()
                        .map_or_else(|| "unknown error".to_string(), Clone::clone)
                },
                |a| format!("expected {expected}, got {a}"),
            );
            ("✗", Color::Red, err_detail)
        }
        TestResult::Skip { reason, .. } => ("⊘", Color::Yellow, reason.clone()),
    };

    // Create styled spans for colored output
    let line = Line::from(vec![
        Span::raw(" "),
        Span::styled(symbol, Style::default().fg(symbol_color)),
        Span::raw(" "),
        Span::styled(name.to_string(), Style::default().fg(cat_color)),
        Span::raw(" "),
        Span::styled(detail, Style::default().fg(Color::DarkGray)),
    ]);

    ListItem::new(line)
}

/// Draws the detail pane for the selected test.
fn draw_details(frame: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == ActivePanel::Details;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let content = app.selected_result().map_or_else(
        || "No test selected.\n\nUse ↑/↓ or j/k to navigate.".to_string(),
        format_detail_content,
    );

    let detail = Paragraph::new(content).wrap(Wrap { trim: false }).block(
        Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(detail, area);
}

/// Formats the detail content for a test result.
fn format_detail_content(result: &TestResult) -> String {
    match result {
        TestResult::Pass {
            name,
            formula,
            expected,
            actual,
        } => {
            format!(
                "Test: {name}\n\n\
                 Status: ✓ PASSED\n\n\
                 Formula:\n  {formula}\n\n\
                 Expected: {expected}\n\
                 Actual:   {actual}"
            )
        }
        TestResult::Fail {
            name,
            formula,
            expected,
            actual,
            error,
        } => {
            let mut s = format!(
                "Test: {name}\n\n\
                 Status: ✗ FAILED\n\n\
                 Formula:\n  {formula}\n\n\
                 Expected: {expected}"
            );
            if let Some(a) = actual {
                let _ = write!(s, "\nActual:   {a}");
            }
            if let Some(e) = error {
                let _ = write!(s, "\n\nError:\n  {e}");
            }
            s
        }
        TestResult::Skip { name, reason } => {
            format!(
                "Test: {name}\n\n\
                 Status: ⊘ SKIPPED\n\n\
                 Reason: {reason}"
            )
        }
    }
}

/// Draws the statistics panel with pass/fail distribution bar.
fn draw_stats(frame: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == ActivePanel::Stats;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let run_state = if app.done { "Done" } else { "Running" };
    let total = app.passed + app.failed + app.skipped;

    // Create a visual bar showing pass/fail ratio
    let bar_width = 20_usize;
    let (pass_chars, fail_chars) = if total > 0 {
        // Calculate pass ratio using integer math to avoid precision loss warnings
        let pass_w = (app.passed * bar_width) / total;
        (pass_w, bar_width - pass_w)
    } else {
        (0, bar_width)
    };

    let bar = format!("[{}{}]", "█".repeat(pass_chars), "░".repeat(fail_chars));

    let line1 = Line::from(vec![
        Span::raw(format!("{run_state}: ")),
        Span::styled(format!("{}", app.passed), Style::default().fg(Color::Green)),
        Span::raw(" pass, "),
        Span::styled(format!("{}", app.failed), Style::default().fg(Color::Red)),
        Span::raw(" fail, "),
        Span::styled(
            format!("{}", app.skipped),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw(" skip"),
    ]);

    let line2 = Line::from(vec![Span::styled(bar, Style::default().fg(Color::Green))]);

    let widget = Paragraph::new(vec![line1, line2])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Stats ")
                .borders(Borders::ALL)
                .border_style(border_style),
        );
    frame.render_widget(widget, area);
}

/// Draws the footer with key hints.
fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let content = match app.input_mode {
        InputMode::Search => Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Cyan)),
            Span::raw(&app.search_query),
            Span::styled("█", Style::default().fg(Color::Cyan)),
            Span::raw(" │ Enter:confirm │ Esc:cancel"),
        ]),
        InputMode::Normal => {
            let hints = if app.done {
                "↑/↓:nav │ Tab:panel │ 1/2/3:filter │ /:search │ q/Enter:exit"
            } else {
                "↑/↓:nav │ Tab:panel │ 1/2/3:filter │ /:search │ q:quit"
            };
            Line::from(hints)
        }
    };

    let style = if app.input_mode == InputMode::Search {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let footer = Paragraph::new(content)
        .style(style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, area);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─────────────────────────────────────────────────────────────────────────
    // FilterMode Tests
    // ─────────────────────────────────────────────────────────────────────────

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

    // ─────────────────────────────────────────────────────────────────────────
    // ActivePanel Tests
    // ─────────────────────────────────────────────────────────────────────────

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

    // ─────────────────────────────────────────────────────────────────────────
    // App Tests
    // ─────────────────────────────────────────────────────────────────────────

    fn make_pass_result(name: &str) -> TestResult {
        TestResult::Pass {
            name: name.to_string(),
            formula: "=TEST()".to_string(),
            expected: 42.0,
            actual: 42.0,
        }
    }

    fn make_fail_result(name: &str) -> TestResult {
        TestResult::Fail {
            name: name.to_string(),
            formula: "=FAIL()".to_string(),
            expected: 42.0,
            actual: Some(0.0),
            error: None,
        }
    }

    fn make_skip_result(name: &str) -> TestResult {
        TestResult::Skip {
            name: name.to_string(),
            reason: "not implemented".to_string(),
        }
    }

    #[test]
    fn app_new_initializes_correctly() {
        let app = App::new(10);
        assert_eq!(app.total_tests, 10);
        assert_eq!(app.current_test, 0);
        assert_eq!(app.passed, 0);
        assert_eq!(app.failed, 0);
        assert_eq!(app.skipped, 0);
        assert!(!app.done);
        assert!(app.running);
        assert!(app.results.is_empty());
    }

    #[test]
    fn app_add_result_updates_counts() {
        let mut app = App::new(5);

        app.add_result(make_pass_result("test1"));
        assert_eq!(app.passed, 1);
        assert_eq!(app.failed, 0);
        assert_eq!(app.current_test, 1);

        app.add_result(make_fail_result("test2"));
        assert_eq!(app.passed, 1);
        assert_eq!(app.failed, 1);
        assert_eq!(app.current_test, 2);

        app.add_result(make_skip_result("test3"));
        assert_eq!(app.skipped, 1);
        assert_eq!(app.current_test, 3);
    }

    #[test]
    fn app_mark_done() {
        let mut app = App::new(1);
        assert!(!app.done);
        app.mark_done();
        assert!(app.done);
        assert!(!app.running);
    }

    #[test]
    fn app_progress_percent() {
        let mut app = App::new(4);
        assert_eq!(app.progress_percent(), 0);

        app.add_result(make_pass_result("test1"));
        assert_eq!(app.progress_percent(), 25);

        app.add_result(make_pass_result("test2"));
        assert_eq!(app.progress_percent(), 50);

        app.add_result(make_pass_result("test3"));
        assert_eq!(app.progress_percent(), 75);

        app.add_result(make_pass_result("test4"));
        assert_eq!(app.progress_percent(), 100);
    }

    #[test]
    fn app_progress_percent_zero_total() {
        let app = App::new(0);
        assert_eq!(app.progress_percent(), 0);
    }

    #[test]
    fn app_filter_all() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("test1"));
        app.add_result(make_fail_result("test2"));
        app.add_result(make_skip_result("test3"));

        app.set_filter(FilterMode::All);
        assert_eq!(app.filtered_results().len(), 3);
    }

    #[test]
    fn app_filter_passed() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("test1"));
        app.add_result(make_fail_result("test2"));
        app.add_result(make_pass_result("test3"));

        app.set_filter(FilterMode::Passed);
        let filtered = app.filtered_results();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| r.is_pass()));
    }

    #[test]
    fn app_filter_failed() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("test1"));
        app.add_result(make_fail_result("test2"));
        app.add_result(make_fail_result("test3"));

        app.set_filter(FilterMode::Failed);
        let filtered = app.filtered_results();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| r.is_fail()));
    }

    #[test]
    fn app_navigation() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("test1"));
        app.add_result(make_pass_result("test2"));
        app.add_result(make_pass_result("test3"));

        // Should auto-select first item
        assert_eq!(app.list_state.selected(), Some(0));

        app.select_next();
        assert_eq!(app.list_state.selected(), Some(1));

        app.select_next();
        assert_eq!(app.list_state.selected(), Some(2));

        // Should not go past end
        app.select_next();
        assert_eq!(app.list_state.selected(), Some(2));

        app.select_previous();
        assert_eq!(app.list_state.selected(), Some(1));

        app.select_previous();
        assert_eq!(app.list_state.selected(), Some(0));

        // Should not go before start
        app.select_previous();
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn app_navigation_empty() {
        let mut app = App::new(0);
        app.select_next();
        assert_eq!(app.list_state.selected(), None);
        app.select_previous();
        assert_eq!(app.list_state.selected(), None);
    }

    #[test]
    fn app_panel_switching() {
        let mut app = App::new(0);
        assert_eq!(app.active_panel, ActivePanel::Results);

        app.next_panel();
        assert_eq!(app.active_panel, ActivePanel::Details);

        app.next_panel();
        assert_eq!(app.active_panel, ActivePanel::Stats);

        app.next_panel();
        assert_eq!(app.active_panel, ActivePanel::Results);

        app.prev_panel();
        assert_eq!(app.active_panel, ActivePanel::Stats);
    }

    #[test]
    fn app_selected_result() {
        let mut app = App::new(2);
        assert!(app.selected_result().is_none());

        app.add_result(make_pass_result("test1"));
        let selected = app.selected_result();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().name(), "test1");
    }

    #[test]
    fn app_filter_resets_selection() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("test1"));
        app.add_result(make_fail_result("test2"));
        app.add_result(make_pass_result("test3"));

        app.select_next();
        app.select_next();

        // Switching filter should reset selection to 0
        app.set_filter(FilterMode::Passed);
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn app_filter_empty_clears_selection() {
        let mut app = App::new(2);
        app.add_result(make_pass_result("test1"));
        app.add_result(make_pass_result("test2"));

        app.set_filter(FilterMode::Failed);
        assert_eq!(app.list_state.selected(), None);
        assert!(app.filtered_results().is_empty());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Format Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn format_result_item_pass() {
        let result = make_pass_result("math.ABS");
        let item = format_result_item(&result);
        // ListItem doesn't expose content directly, but we can verify it doesn't panic
        assert!(!format!("{item:?}").is_empty());
    }

    #[test]
    fn format_result_item_fail() {
        let result = make_fail_result("math.SQRT");
        let item = format_result_item(&result);
        assert!(!format!("{item:?}").is_empty());
    }

    #[test]
    fn format_result_item_skip() {
        let result = make_skip_result("math.TBD");
        let item = format_result_item(&result);
        assert!(!format!("{item:?}").is_empty());
    }

    #[test]
    fn format_detail_content_pass() {
        let result = make_pass_result("test");
        let content = format_detail_content(&result);
        assert!(content.contains("PASSED"));
        assert!(content.contains("test"));
        assert!(content.contains("42"));
    }

    #[test]
    fn format_detail_content_fail() {
        let result = make_fail_result("test");
        let content = format_detail_content(&result);
        assert!(content.contains("FAILED"));
        assert!(content.contains("test"));
        assert!(content.contains("Expected"));
    }

    #[test]
    fn format_detail_content_fail_with_error() {
        let result = TestResult::Fail {
            name: "test".to_string(),
            formula: "=ERR()".to_string(),
            expected: 1.0,
            actual: None,
            error: Some("Something went wrong".to_string()),
        };
        let content = format_detail_content(&result);
        assert!(content.contains("Error:"));
        assert!(content.contains("Something went wrong"));
    }

    #[test]
    fn format_detail_content_skip() {
        let result = make_skip_result("test");
        let content = format_detail_content(&result);
        assert!(content.contains("SKIPPED"));
        assert!(content.contains("not implemented"));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Search Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn app_search_mode_enter_exit() {
        let mut app = App::new(0);
        assert_eq!(app.input_mode, InputMode::Normal);

        app.enter_search_mode();
        assert_eq!(app.input_mode, InputMode::Search);

        app.exit_search_mode();
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn app_search_push_pop() {
        let mut app = App::new(0);
        app.enter_search_mode();

        app.search_push('t');
        assert_eq!(app.search_query, "t");

        app.search_push('e');
        app.search_push('s');
        app.search_push('t');
        assert_eq!(app.search_query, "test");

        app.search_pop();
        assert_eq!(app.search_query, "tes");
    }

    #[test]
    fn app_search_filters_results() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("math.ABS"));
        app.add_result(make_pass_result("text.CONCAT"));
        app.add_result(make_pass_result("math.SQRT"));

        assert_eq!(app.filtered_results().len(), 3);

        app.search_push('m');
        app.search_push('a');
        app.search_push('t');
        app.search_push('h');

        let filtered = app.filtered_results();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|r| r.name().contains("math")));
    }

    #[test]
    fn app_search_case_insensitive() {
        let mut app = App::new(2);
        app.add_result(make_pass_result("Math.ABS"));
        app.add_result(make_pass_result("text.concat"));

        app.search_push('M');
        app.search_push('A');
        app.search_push('T');
        app.search_push('H');

        assert_eq!(app.filtered_results().len(), 1);
    }

    #[test]
    fn app_search_exit_clears_query() {
        let mut app = App::new(2);
        app.add_result(make_pass_result("test1"));
        app.add_result(make_pass_result("test2"));

        app.enter_search_mode();
        app.search_push('1');
        assert_eq!(app.filtered_results().len(), 1);

        app.exit_search_mode();
        assert!(app.search_query.is_empty());
        assert_eq!(app.filtered_results().len(), 2);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Category Color Tests
    // ─────────────────────────────────────────────────────────────────────────

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

    // ─────────────────────────────────────────────────────────────────────────
    // Input Mode Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn input_mode_default() {
        assert_eq!(InputMode::default(), InputMode::Normal);
    }
}
