//! TUI interface using ratatui
//!
//! Provides an interactive terminal UI for running E2E validation tests.
//! - v1.6.0: Performance benchmarks (tests/sec, elapsed time)
//! - v1.7.0: Function coverage report (48/48 functions validated)
//! - v1.8.0: R&D preview teaser (shows locked functions count)
//! - v1.9.0: Side-by-side comparison mode (toggle with `c` key)
//! - v2.1.0: Perf mode (p key - parallel forge calculate, skip Gnumeric)
//! - v2.1.0: Batch mode (b key - single XLSX, one Gnumeric call)

mod app;
mod draw;
mod state;

pub use app::App;
pub use state::{FilterMode, InputMode};

use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;

use crate::runner::TestRunner;
use draw::draw_ui;

/// Runs the TUI interface.
pub fn run(runner: &TestRunner) -> anyhow::Result<bool> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let result = run_app(&mut terminal, runner);
    let _ = disable_raw_mode();
    let _ = stdout().execute(LeaveAlternateScreen);
    result
}

fn run_tests(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runner: &TestRunner,
    app: &mut App,
    perf_mode: bool,
    batch_mode: bool,
) -> anyhow::Result<bool> {
    if batch_mode {
        // Batch mode: single XLSX for all tests
        terminal.draw(|frame| draw_ui(frame, app))?;
        let results = runner.run_batch();
        for result in results {
            app.add_result(result);
        }
        terminal.draw(|frame| draw_ui(frame, app))?;
        app.mark_done();
        return Ok(true);
    }

    if perf_mode {
        // Perf mode: parallel execution with rayon
        terminal.draw(|frame| draw_ui(frame, app))?;
        let results = runner.run_perf_parallel();
        for result in results {
            app.add_result(result);
        }
        terminal.draw(|frame| draw_ui(frame, app))?;
        app.mark_done();
        return Ok(true);
    }

    // Normal mode: sequential with Gnumeric validation
    // First, add all skip results
    for skip_case in runner.skip_cases() {
        app.add_result(crate::types::TestResult::Skip {
            name: skip_case.name.clone(),
            reason: skip_case.reason.clone(),
        });
        terminal.draw(|frame| draw_ui(frame, app))?;
    }

    // Then run actual tests
    let test_cases = runner.test_cases().to_vec();
    for test_case in test_cases {
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(false);
                }
            }
        }
        terminal.draw(|frame| draw_ui(frame, app))?;
        let result = runner.run_test(&test_case);
        app.add_result(result);
        terminal.draw(|frame| draw_ui(frame, app))?;
    }

    app.mark_done();
    Ok(true)
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runner: &TestRunner,
) -> anyhow::Result<bool> {
    let total = runner.total_tests();
    let mut app = App::new(total);
    let mut perf_mode = false;
    let mut batch_mode = false;

    // Initial run (full validation)
    run_tests(terminal, runner, &mut app, perf_mode, batch_mode)?;

    loop {
        terminal.draw(|frame| draw_ui(frame, &mut app))?;
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(app.failed == 0),
                            KeyCode::Enter if app.done => return Ok(app.failed == 0),
                            KeyCode::Char('/') => app.enter_search_mode(),
                            KeyCode::Char('s') if app.done => {
                                if let Err(e) = app.save_to_json() {
                                    app.set_status(format!("Error: {e}"));
                                }
                            }
                            KeyCode::Char('p') if app.done => {
                                // Toggle perf mode and rerun
                                perf_mode = !perf_mode;
                                batch_mode = false;
                                app.reset(perf_mode, batch_mode);
                                let mode_name = if perf_mode { "PERF" } else { "FULL" };
                                app.set_status(format!("Rerunning in {mode_name} mode..."));
                                run_tests(terminal, runner, &mut app, perf_mode, batch_mode)?;
                            }
                            KeyCode::Char('b') if app.done => {
                                // Toggle batch mode and rerun
                                batch_mode = !batch_mode;
                                perf_mode = false;
                                app.reset(perf_mode, batch_mode);
                                let mode_name = if batch_mode { "BATCH" } else { "FULL" };
                                app.set_status(format!("Rerunning in {mode_name} mode..."));
                                run_tests(terminal, runner, &mut app, perf_mode, batch_mode)?;
                            }
                            KeyCode::Char('c') => app.toggle_comparison_mode(),
                            KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
                            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                            KeyCode::Tab => app.next_panel(),
                            KeyCode::BackTab => app.prev_panel(),
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
