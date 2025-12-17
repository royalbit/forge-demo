//! TUI application state - App struct and all its methods.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::widgets::ListState;

use super::state::{ActivePanel, FilterMode, InputMode};
use crate::types::TestResult;

/// Duration to show status messages.
const STATUS_MESSAGE_DURATION: Duration = Duration::from_secs(3);

/// Demo mode function count (v1.0.0 schema).
pub const DEMO_FUNCTION_COUNT: usize = 48;
/// Enterprise function count (full forge).
pub const ENTERPRISE_FUNCTION_COUNT: usize = 173;

/// Main application state for the TUI.
pub struct App {
    /// All test results collected so far.
    pub(super) results: Vec<TestResult>,
    /// Index of the currently executing test.
    pub(super) current_test: usize,
    /// Total number of tests to run.
    pub(super) total_tests: usize,
    /// Whether tests are still running.
    pub(super) running: bool,
    /// Whether all tests have completed.
    pub(super) done: bool,
    /// Count of passed tests.
    pub(super) passed: usize,
    /// Count of failed tests.
    pub(super) failed: usize,
    /// Count of skipped tests.
    pub(super) skipped: usize,
    /// Currently active panel.
    pub(super) active_panel: ActivePanel,
    /// Current filter mode.
    pub(super) filter_mode: FilterMode,
    /// State for the results list (selection, scroll offset).
    pub(super) list_state: ListState,
    /// Cached filtered indices for the current filter mode.
    pub(super) filtered_indices: Vec<usize>,
    /// Current input mode (normal or search).
    pub(super) input_mode: InputMode,
    /// Search query string.
    pub(super) search_query: String,
    /// Status message to display (with expiration time).
    status_message: Option<(String, Instant)>,
    /// Time when tests started running.
    pub(super) start_time: Option<Instant>,
    /// Total execution time after tests complete.
    pub(super) total_duration: Option<Duration>,
    /// Function coverage by category (category -> set of function names).
    function_coverage: HashMap<String, Vec<String>>,
    /// Whether comparison mode is active (toggle with 'c' key).
    pub(super) comparison_mode: bool,
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
            status_message: None,
            start_time: Some(Instant::now()),
            total_duration: None,
            function_coverage: HashMap::new(),
            comparison_mode: false,
        }
    }

    /// Adds a test result and updates statistics.
    pub fn add_result(&mut self, result: TestResult) {
        match &result {
            TestResult::Pass { .. } => self.passed += 1,
            TestResult::Fail { .. } => self.failed += 1,
            TestResult::Skip { .. } => self.skipped += 1,
        }
        self.track_function_coverage(result.name());
        self.results.push(result);
        self.current_test += 1;
        self.update_filtered_indices();
        if self.list_state.selected().is_none() && !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    /// Marks the app as done running tests.
    pub fn mark_done(&mut self) {
        self.running = false;
        self.done = true;
        if let Some(start) = self.start_time {
            self.total_duration = Some(start.elapsed());
        }
    }

    fn track_function_coverage(&mut self, name: &str) {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() >= 2 {
            let category = parts[0].to_string();
            let function = parts[1..].join(".");
            self.function_coverage
                .entry(category)
                .or_default()
                .push(function);
        }
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn tests_per_second(&self) -> Option<f64> {
        let duration = self
            .total_duration
            .or_else(|| self.start_time.map(|s| s.elapsed()))?;
        let secs = duration.as_secs_f64();
        if secs > 0.0 {
            Some(self.current_test as f64 / secs)
        } else {
            None
        }
    }

    #[allow(clippy::option_if_let_else)]
    pub fn elapsed_time(&self) -> String {
        let duration = self
            .total_duration
            .or_else(|| self.start_time.map(|s| s.elapsed()));
        match duration {
            Some(d) => {
                let millis = d.as_millis();
                if millis < 1000 {
                    format!("{millis}ms")
                } else {
                    format!("{:.2}s", d.as_secs_f64())
                }
            }
            None => "—".to_string(),
        }
    }

    pub fn unique_functions_tested(&self) -> usize {
        self.function_coverage.values().map(Vec::len).sum()
    }

    pub fn coverage_by_category(&self) -> Vec<(&str, usize)> {
        let mut cats: Vec<_> = self
            .function_coverage
            .iter()
            .map(|(k, v)| (k.as_str(), v.len()))
            .collect();
        cats.sort_by(|a, b| b.1.cmp(&a.1));
        cats
    }

    pub fn toggle_comparison_mode(&mut self) {
        self.comparison_mode = !self.comparison_mode;
        let mode = if self.comparison_mode { "ON" } else { "OFF" };
        self.set_status(format!("Comparison mode: {mode}"));
    }

    fn update_filtered_indices(&mut self) {
        let query_lower = self.search_query.to_lowercase();
        self.filtered_indices = self
            .results
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                let passes_filter = match self.filter_mode {
                    FilterMode::All => true,
                    FilterMode::Passed => r.is_pass(),
                    FilterMode::Failed => r.is_fail(),
                };
                let passes_search =
                    query_lower.is_empty() || r.name().to_lowercase().contains(&query_lower);
                passes_filter && passes_search
            })
            .map(|(i, _)| i)
            .rev()
            .collect();
    }

    pub const fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
    }

    pub fn exit_search_mode(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.update_filtered_indices();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filtered_indices();
        if self.filtered_indices.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.update_filtered_indices();
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some((message.into(), Instant::now()));
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_ref().and_then(|(msg, created)| {
            if created.elapsed() < STATUS_MESSAGE_DURATION {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    pub fn save_to_json(&mut self) -> Result<PathBuf, String> {
        let filename = format!(
            "forge-e2e-results-{}.json",
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let path = PathBuf::from(&filename);
        let output = serde_json::json!({
            "timestamp": chrono::Local::now().to_rfc3339(),
            "summary": { "total": self.results.len(), "passed": self.passed, "failed": self.failed, "skipped": self.skipped },
            "results": &self.results,
        });
        let json = serde_json::to_string_pretty(&output)
            .map_err(|e| format!("Failed to serialize: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("Failed to write file: {e}"))?;
        self.set_status(format!("Saved to {filename}"));
        Ok(path)
    }

    pub fn set_filter(&mut self, mode: FilterMode) {
        if self.filter_mode != mode {
            self.filter_mode = mode;
            self.update_filtered_indices();
            if self.filtered_indices.is_empty() {
                self.list_state.select(None);
            } else {
                self.list_state.select(Some(0));
            }
        }
    }

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

    pub fn selected_result(&self) -> Option<&TestResult> {
        self.list_state
            .selected()
            .and_then(|i| self.filtered_indices.get(i))
            .and_then(|&idx| self.results.get(idx))
    }

    pub fn filtered_results(&self) -> Vec<&TestResult> {
        self.filtered_indices
            .iter()
            .filter_map(|&i| self.results.get(i))
            .collect()
    }

    pub const fn next_panel(&mut self) {
        self.active_panel = self.active_panel.next();
    }
    pub const fn prev_panel(&mut self) {
        self.active_panel = self.active_panel.prev();
    }

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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!app.done);
    }
    #[test]
    fn app_add_result_updates_counts() {
        let mut app = App::new(5);
        app.add_result(make_pass_result("test1"));
        assert_eq!(app.passed, 1);
        app.add_result(make_fail_result("test2"));
        assert_eq!(app.failed, 1);
        app.add_result(make_skip_result("test3"));
        assert_eq!(app.skipped, 1);
    }
    #[test]
    fn app_mark_done() {
        let mut app = App::new(1);
        app.mark_done();
        assert!(app.done);
        assert!(app.total_duration.is_some());
    }
    #[test]
    fn app_progress_percent() {
        let mut app = App::new(4);
        assert_eq!(app.progress_percent(), 0);
        app.add_result(make_pass_result("test1"));
        assert_eq!(app.progress_percent(), 25);
    }
    #[test]
    fn app_filter_all() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("t1"));
        app.add_result(make_fail_result("t2"));
        app.set_filter(FilterMode::All);
        assert_eq!(app.filtered_results().len(), 2);
    }
    #[test]
    fn app_filter_passed() {
        let mut app = App::new(2);
        app.add_result(make_pass_result("t1"));
        app.add_result(make_fail_result("t2"));
        app.set_filter(FilterMode::Passed);
        assert_eq!(app.filtered_results().len(), 1);
    }
    #[test]
    fn app_navigation() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("t1"));
        app.add_result(make_pass_result("t2"));
        app.select_next();
        assert_eq!(app.list_state.selected(), Some(1));
        app.select_previous();
        assert_eq!(app.list_state.selected(), Some(0));
    }
    #[test]
    fn app_panel_switching() {
        let mut app = App::new(0);
        app.next_panel();
        assert_eq!(app.active_panel, ActivePanel::Details);
    }
    #[test]
    fn app_search_mode() {
        let mut app = App::new(2);
        app.add_result(make_pass_result("math.ABS"));
        app.add_result(make_pass_result("text.CONCAT"));
        app.enter_search_mode();
        app.search_push('m');
        assert_eq!(app.filtered_results().len(), 1);
        app.exit_search_mode();
        assert_eq!(app.filtered_results().len(), 2);
    }
    #[test]
    fn app_status_message() {
        let mut app = App::new(0);
        app.set_status("Test");
        assert_eq!(app.status_message(), Some("Test"));
    }
    #[test]
    fn app_tests_per_second() {
        let mut app = App::new(2);
        app.add_result(make_pass_result("t1"));
        assert!(app.tests_per_second().is_some());
    }
    #[test]
    fn app_coverage() {
        let mut app = App::new(3);
        app.add_result(make_pass_result("math.ABS"));
        app.add_result(make_pass_result("math.SQRT"));
        app.add_result(make_pass_result("text.CONCAT"));
        assert_eq!(app.unique_functions_tested(), 3);
    }
    #[test]
    fn app_comparison_mode() {
        let mut app = App::new(0);
        assert!(!app.comparison_mode);
        app.toggle_comparison_mode();
        assert!(app.comparison_mode);
    }
}
