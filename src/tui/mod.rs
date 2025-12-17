//! TUI interface using ratatui
//!
//! Provides an interactive terminal UI for running E2E validation tests.
//! - v1.6.0: Performance benchmarks (tests/sec, elapsed time)
//! - v1.7.0: Function coverage report (48/48 functions validated)
//! - v1.8.0: Enterprise teaser (shows locked functions count)
//! - v1.9.0: Side-by-side comparison mode (toggle with `c` key)

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

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runner: &TestRunner,
) -> anyhow::Result<bool> {
    let total = runner.total_tests();
    let mut app = App::new(total);
    let test_cases = runner.test_cases().to_vec();

    for test_case in test_cases {
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(false);
                }
            }
        }
        terminal.draw(|frame| draw_ui(frame, &mut app))?;
        let result = runner.run_test(&test_case);
        app.add_result(result);
        terminal.draw(|frame| draw_ui(frame, &mut app))?;
    }

    app.mark_done();

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
