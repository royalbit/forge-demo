//! TUI interface using ratatui

use std::io::{self, stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

use crate::runner::TestRunner;
use crate::types::TestResult;

/// Run the TUI
pub fn run(mut runner: TestRunner) -> anyhow::Result<bool> {
    // Setup terminal
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run_app(&mut terminal, &mut runner);

    // Restore terminal
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    result
}

/// Main app state
struct App {
    results: Vec<TestResult>,
    current_test: usize,
    total_tests: usize,
    running: bool,
    done: bool,
    passed: usize,
    failed: usize,
}

impl App {
    fn new(total: usize) -> Self {
        Self {
            results: Vec::new(),
            current_test: 0,
            total_tests: total,
            running: true,
            done: false,
            passed: 0,
            failed: 0,
        }
    }

    fn add_result(&mut self, result: TestResult) {
        if result.is_pass() {
            self.passed += 1;
        } else if result.is_fail() {
            self.failed += 1;
        }
        self.results.push(result);
        self.current_test += 1;
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    runner: &mut TestRunner,
) -> anyhow::Result<bool> {
    let total = runner.total_tests();
    let mut app = App::new(total);
    let test_cases = runner.test_cases().to_vec();

    // Run tests one by one
    for test_case in test_cases {
        // Check for quit
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(false);
                }
            }
        }

        // Draw current state
        terminal.draw(|frame| draw_ui(frame, &app))?;

        // Run test
        let result = runner.run_test(&test_case);
        app.add_result(result);

        // Draw updated state
        terminal.draw(|frame| draw_ui(frame, &app))?;
    }

    app.running = false;
    app.done = true;

    // Final draw and wait for exit
    loop {
        terminal.draw(|frame| draw_ui(frame, &app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
                            return Ok(app.failed == 0);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn draw_ui(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Progress bar
            Constraint::Length(3), // Stats
            Constraint::Min(10),   // Results list
            Constraint::Length(3), // Footer
        ])
        .split(frame.area());

    // Title
    let title = Paragraph::new("forge-e2e: E2E Validation Suite")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Progress bar
    let progress = if app.total_tests > 0 {
        (app.current_test as f64 / app.total_tests as f64 * 100.0) as u16
    } else {
        0
    };
    let gauge = Gauge::default()
        .block(Block::default().title("Progress").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(progress)
        .label(format!("{}/{} tests", app.current_test, app.total_tests));
    frame.render_widget(gauge, chunks[1]);

    // Stats
    let stats_text = if app.done {
        format!("Completed: {} passed, {} failed", app.passed, app.failed)
    } else {
        format!("Running: {} passed, {} failed", app.passed, app.failed)
    };
    let stats_style = if app.failed > 0 {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Green)
    };
    let stats = Paragraph::new(stats_text)
        .style(stats_style)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(stats, chunks[2]);

    // Results list (show last 20)
    let items: Vec<ListItem> = app
        .results
        .iter()
        .rev()
        .take(20)
        .map(|r| {
            let (symbol, style, text) = match r {
                TestResult::Pass { name, actual, .. } => (
                    "✓",
                    Style::default().fg(Color::Green),
                    format!("{} = {}", name, actual),
                ),
                TestResult::Fail {
                    name,
                    expected,
                    actual,
                    error,
                    ..
                } => {
                    let detail = if let Some(a) = actual {
                        format!("expected {}, got {}", expected, a)
                    } else if let Some(e) = error {
                        e.clone()
                    } else {
                        "unknown error".to_string()
                    };
                    (
                        "✗",
                        Style::default().fg(Color::Red),
                        format!("{}: {}", name, detail),
                    )
                }
                TestResult::Skip { name, reason, .. } => (
                    "⊘",
                    Style::default().fg(Color::Yellow),
                    format!("{}: {}", name, reason),
                ),
            };
            ListItem::new(format!(" {} {}", symbol, text)).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().title("Results").borders(Borders::ALL));
    frame.render_widget(list, chunks[3]);

    // Footer
    let footer_text = if app.done {
        "Press [q] or [Enter] to exit"
    } else {
        "Running tests... Press [q] to quit"
    };
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(footer, chunks[4]);
}
