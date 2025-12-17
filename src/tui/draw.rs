//! TUI drawing functions - renders all UI components.

use super::app::{App, DEMO_FUNCTION_COUNT, ENTERPRISE_FUNCTION_COUNT};
use super::state::{category_color, ActivePanel, FilterMode, InputMode};
use crate::types::TestResult;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Wrap},
};
use std::fmt::Write as _;

pub fn draw_ui(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(4),
            Constraint::Length(3),
        ])
        .split(area);

    draw_title(frame, main_chunks[0]);
    draw_progress(frame, main_chunks[1], app);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[2]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(5)])
        .split(content_chunks[1]);

    if app.comparison_mode {
        draw_comparison_view(frame, content_chunks[0], app);
    } else {
        draw_results_list(frame, content_chunks[0], app);
    }
    draw_details(frame, right_chunks[0], app);
    draw_stats(frame, right_chunks[1], app);
    draw_coverage_bar(frame, main_chunks[3], app);
    draw_footer(frame, main_chunks[4], app);
}

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

fn draw_results_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let is_active = app.active_panel == ActivePanel::Results;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let format_filter = |mode: FilterMode| {
        if app.filter_mode == mode {
            mode.label().to_uppercase()
        } else {
            mode.label().to_string()
        }
    };
    let filter_label = format!(
        " Results [{}:{} {}:{} {}:{}] ",
        FilterMode::All.shortcut(),
        format_filter(FilterMode::All),
        FilterMode::Passed.shortcut(),
        format_filter(FilterMode::Passed),
        FilterMode::Failed.shortcut(),
        format_filter(FilterMode::Failed),
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

fn format_detail_content(result: &TestResult) -> String {
    match result {
        TestResult::Pass { name, formula, expected, actual } =>
            format!("Test: {name}\n\nStatus: ✓ PASSED\n\nFormula:\n  {formula}\n\nExpected: {expected}\nActual:   {actual}"),
        TestResult::Fail { name, formula, expected, actual, error } => {
            let mut s = format!("Test: {name}\n\nStatus: ✗ FAILED\n\nFormula:\n  {formula}\n\nExpected: {expected}");
            if let Some(a) = actual { let _ = write!(s, "\nActual:   {a}"); }
            if let Some(e) = error { let _ = write!(s, "\n\nError:\n  {e}"); }
            s
        }
        TestResult::Skip { name, reason } => format!("Test: {name}\n\nStatus: ⊘ SKIPPED\n\nReason: {reason}"),
    }
}

fn draw_stats(frame: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == ActivePanel::Stats;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let run_state = if app.done { "Done" } else { "Running" };
    let total = app.passed + app.failed + app.skipped;
    let bar_width = 20_usize;
    let (pass_chars, fail_chars) = if total > 0 {
        let pass_w = (app.passed * bar_width) / total;
        (pass_w, bar_width - pass_w)
    } else {
        (0, bar_width)
    };
    let bar = format!("[{}{}]", "█".repeat(pass_chars), "░".repeat(fail_chars));
    let perf_info = app.tests_per_second().map_or_else(String::new, |tps| {
        format!(" | {:.1} tests/sec | {}", tps, app.elapsed_time())
    });
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
        Span::styled(perf_info, Style::default().fg(Color::DarkGray)),
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

fn draw_coverage_bar(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);
    let unique_funcs = app.unique_functions_tested();
    let coverage_pct = if DEMO_FUNCTION_COUNT > 0 {
        (unique_funcs * 100) / DEMO_FUNCTION_COUNT
    } else {
        0
    };
    let categories = app.coverage_by_category();
    let cat_summary: String = categories
        .iter()
        .take(4)
        .map(|(cat, count)| format!("{cat}:{count}"))
        .collect::<Vec<_>>()
        .join(" ");
    let coverage_line1 = Line::from(vec![
        Span::styled("Coverage: ", Style::default().fg(Color::Cyan)),
        Span::styled(
            format!("{unique_funcs}/{DEMO_FUNCTION_COUNT}"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" ({coverage_pct}%) ")),
        Span::styled(cat_summary, Style::default().fg(Color::DarkGray)),
    ]);
    let coverage_widget = Paragraph::new(vec![coverage_line1])
        .block(
            Block::default()
                .title(" Function Coverage ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Left);
    frame.render_widget(coverage_widget, chunks[0]);
    let locked_count = ENTERPRISE_FUNCTION_COUNT - DEMO_FUNCTION_COUNT;
    let teaser_line1 = Line::from(vec![
        Span::styled("Demo Mode", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        Span::styled(
            format!("+{locked_count} functions"),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw(" in "),
        Span::styled(
            "Enterprise",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let teaser_widget = Paragraph::new(vec![teaser_line1])
        .block(
            Block::default()
                .title(" Upgrade ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);
    frame.render_widget(teaser_widget, chunks[1]);
}

fn draw_comparison_view(frame: &mut Frame, area: Rect, app: &App) {
    let is_active = app.active_panel == ActivePanel::Results;
    let border_style = if is_active {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let items: Vec<(String, String, String, bool)> = app
        .filtered_results()
        .iter()
        .map(|r| {
            let name = r.name().to_string();
            match r {
                TestResult::Pass {
                    expected, actual, ..
                } => (name, format!("{expected}"), format!("{actual}"), true),
                TestResult::Fail {
                    expected, actual, ..
                } => {
                    let actual_str = actual.map_or_else(|| "ERR".to_string(), |a| format!("{a}"));
                    (name, format!("{expected}"), actual_str, false)
                }
                TestResult::Skip { reason, .. } => (name, "—".to_string(), reason.clone(), false),
            }
        })
        .collect();
    let forge_items: Vec<ListItem> = items
        .iter()
        .map(|(name, expected, _, passed)| {
            let color = if *passed { Color::Green } else { Color::Red };
            let symbol = if *passed { "✓" } else { "✗" };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{symbol} "), Style::default().fg(color)),
                Span::raw(format!("{name}: ")),
                Span::styled(expected, Style::default().fg(Color::Cyan)),
            ]))
        })
        .collect();
    let forge_list = List::new(forge_items)
        .block(
            Block::default()
                .title(" Expected (Forge) ")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    let mut list_state = app.list_state.clone();
    frame.render_stateful_widget(forge_list, chunks[0], &mut list_state);
    let gnumeric_items: Vec<ListItem> = items
        .iter()
        .map(|(name, _, actual, passed)| {
            let color = if *passed { Color::Green } else { Color::Red };
            ListItem::new(Line::from(vec![
                Span::raw(format!("{name}: ")),
                Span::styled(actual, Style::default().fg(color)),
            ]))
        })
        .collect();
    let gnumeric_list = List::new(gnumeric_items).block(
        Block::default()
            .title(" Actual (Gnumeric) ")
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(gnumeric_list, chunks[1]);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    if let Some(status) = app.status_message() {
        let footer = Paragraph::new(status)
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(footer, area);
        return;
    }
    let content = match app.input_mode {
        InputMode::Search => Line::from(vec![
            Span::styled("Search: ", Style::default().fg(Color::Cyan)),
            Span::raw(&app.search_query),
            Span::styled("█", Style::default().fg(Color::Cyan)),
            Span::raw(" │ Enter:confirm │ Esc:cancel"),
        ]),
        InputMode::Normal => {
            let hints = if app.done {
                "↑/↓:nav │ Tab:panel │ 1/2/3:filter │ /:search │ c:compare │ s:save │ q:exit"
            } else {
                "↑/↓:nav │ Tab:panel │ 1/2/3:filter │ /:search │ c:compare │ q:quit"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_result_item_pass() {
        let result = TestResult::Pass {
            name: "test".to_string(),
            formula: "=1".to_string(),
            expected: 1.0,
            actual: 1.0,
        };
        let item = format_result_item(&result);
        assert!(format!("{item:?}").contains("test"));
    }
    #[test]
    fn format_result_item_fail() {
        let result = TestResult::Fail {
            name: "test".to_string(),
            formula: "=1".to_string(),
            expected: 1.0,
            actual: Some(2.0),
            error: None,
        };
        let item = format_result_item(&result);
        assert!(format!("{item:?}").contains("test"));
    }
    #[test]
    fn format_detail_content_pass() {
        let result = TestResult::Pass {
            name: "test".to_string(),
            formula: "=1".to_string(),
            expected: 1.0,
            actual: 1.0,
        };
        let content = format_detail_content(&result);
        assert!(content.contains("PASSED"));
    }
    #[test]
    fn format_detail_content_fail() {
        let result = TestResult::Fail {
            name: "test".to_string(),
            formula: "=1".to_string(),
            expected: 1.0,
            actual: Some(2.0),
            error: None,
        };
        let content = format_detail_content(&result);
        assert!(content.contains("FAILED"));
    }
    #[test]
    fn format_detail_content_skip() {
        let result = TestResult::Skip {
            name: "test".to_string(),
            reason: "reason".to_string(),
        };
        let content = format_detail_content(&result);
        assert!(content.contains("SKIPPED"));
    }
}
