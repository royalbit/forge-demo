//! forge-e2e: E2E validation tool for forge-demo.
//!
//! Validates forge-demo calculations against Gnumeric.
//! Default: TUI mode | --all: verbose headless mode (runs all 3 modes)

mod engine;
mod excel;
mod runner;
mod tui;
mod types;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use colored::Colorize;

use crate::engine::SpreadsheetEngine;
use crate::runner::TestRunner;
use crate::types::TestResult;

// ─────────────────────────────────────────────────────────────────────────────
// CLI
// ─────────────────────────────────────────────────────────────────────────────

/// CLI arguments for forge-e2e.
#[derive(Parser)]
#[command(name = "forge-e2e")]
#[command(about = "E2E validation tool for forge-demo")]
#[command(version)]
struct Cli {
    /// Run all tests in verbose headless mode (colored YAML output).
    #[arg(long)]
    all: bool,

    /// Path to test specs directory.
    #[arg(short, long, default_value = "tests/e2e")]
    tests: PathBuf,

    /// Path to forge-demo binary.
    #[arg(short, long, default_value = "bin/forge-demo")]
    binary: PathBuf,
}

// ─────────────────────────────────────────────────────────────────────────────
// Main
// ─────────────────────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Check for spreadsheet engine
    let Some(engine) = SpreadsheetEngine::detect() else {
        eprintln!(
            "{} Gnumeric not found. Install with: brew install gnumeric (macOS) or apt install gnumeric (Linux)",
            "ERROR:".red().bold()
        );
        return ExitCode::FAILURE;
    };

    if cli.all {
        println!(
            "{} {} ({})",
            "Engine:".cyan().bold(),
            SpreadsheetEngine::name(),
            engine.version()
        );
    }

    // Check for forge-demo binary
    if !cli.binary.exists() {
        eprintln!(
            "{} forge-demo binary not found at {}",
            "ERROR:".red().bold(),
            cli.binary.display()
        );
        eprintln!("  Use ./run-demo.sh which handles downloads automatically");
        return ExitCode::FAILURE;
    }

    // Create test runner
    let runner = match TestRunner::new(cli.binary.clone(), engine, cli.tests.clone()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "{} Failed to initialize test runner: {e}",
                "ERROR:".red().bold(),
            );
            return ExitCode::FAILURE;
        }
    };

    // Run tests
    if cli.all {
        run_all_mode(&runner)
    } else {
        run_tui_mode(&runner)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Run Modes
// ─────────────────────────────────────────────────────────────────────────────

/// Runs in verbose headless mode with colored output.
/// Executes all three test modes: Normal (Gnumeric), Perf (parallel), Batch.
#[allow(clippy::too_many_lines)]
fn run_all_mode(runner: &TestRunner) -> ExitCode {
    println!();
    println!("{}", "═".repeat(70).cyan());
    println!("{}", "  forge-e2e: E2E Validation Suite".cyan().bold());
    println!("{}", "═".repeat(70).cyan());

    let mut total_failed = 0;

    // ─────────────────────────────────────────────────────────────────────────
    // Mode 1: Normal (Gnumeric validation)
    // ─────────────────────────────────────────────────────────────────────────
    println!();
    println!(
        "{}",
        "┌─ NORMAL MODE (Gnumeric validation) ─────────────────────────────────┐"
            .cyan()
            .bold()
    );
    let start = Instant::now();
    let results = runner.run_all();
    let elapsed = start.elapsed();

    let (passed, failed, skipped) = print_results(&results);
    total_failed += failed;
    print_summary("Normal", passed, failed, skipped, elapsed);

    // ─────────────────────────────────────────────────────────────────────────
    // Mode 2: Perf (parallel forge calculate)
    // ─────────────────────────────────────────────────────────────────────────
    println!();
    println!(
        "{}",
        "┌─ PERF MODE (parallel forge calculate) ──────────────────────────────┐"
            .cyan()
            .bold()
    );
    let start = Instant::now();
    let results = runner.run_perf_parallel();
    let elapsed = start.elapsed();

    let (passed, failed, skipped) = print_results(&results);
    total_failed += failed;
    print_summary("Perf", passed, failed, skipped, elapsed);

    // ─────────────────────────────────────────────────────────────────────────
    // Mode 3: Batch (single XLSX, one Gnumeric call)
    // ─────────────────────────────────────────────────────────────────────────
    println!();
    println!(
        "{}",
        "┌─ BATCH MODE (single XLSX, one Gnumeric call) ───────────────────────┐"
            .cyan()
            .bold()
    );
    let start = Instant::now();
    let results = runner.run_batch();
    let elapsed = start.elapsed();

    let (passed, failed, skipped) = print_results(&results);
    total_failed += failed;
    print_summary("Batch", passed, failed, skipped, elapsed);

    // ─────────────────────────────────────────────────────────────────────────
    // Final summary
    // ─────────────────────────────────────────────────────────────────────────
    println!();
    println!("{}", "═".repeat(70).cyan());
    if total_failed > 0 {
        println!(
            "  {} {}",
            "FAILED:".red().bold(),
            format!("{total_failed} test(s) failed across all modes").red()
        );
    } else {
        println!(
            "  {} {}",
            "SUCCESS:".green().bold(),
            "All modes passed!".green()
        );
    }
    println!("{}", "═".repeat(70).cyan());
    println!();

    if total_failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Prints test results and returns (passed, failed, skipped) counts.
fn print_results(results: &[TestResult]) -> (usize, usize, usize) {
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for result in results {
        match result {
            TestResult::Pass { name, actual, .. } => {
                println!(
                    "  {} {} = {}",
                    "✓".green().bold(),
                    name.white(),
                    actual.to_string().green()
                );
                passed += 1;
            }
            TestResult::Fail {
                name,
                formula,
                expected,
                actual,
                error,
            } => {
                println!("  {} {}", "✗".red().bold(), name.white());
                println!("      formula:  {}", formula.yellow());
                println!("      expected: {}", expected.to_string().green());
                if let Some(a) = actual {
                    println!("      actual:   {}", a.to_string().red());
                }
                if let Some(e) = error {
                    println!("      error:    {}", e.red());
                }
                failed += 1;
            }
            TestResult::Skip { name, reason } => {
                println!(
                    "  {} {} ({})",
                    "⊘".yellow().bold(),
                    name.white(),
                    reason.yellow()
                );
                skipped += 1;
            }
        }
    }

    (passed, failed, skipped)
}

/// Prints mode summary with timing.
#[allow(clippy::cast_precision_loss)]
fn print_summary(
    mode: &str,
    passed: usize,
    failed: usize,
    skipped: usize,
    elapsed: std::time::Duration,
) {
    let total = passed + failed + skipped;
    let tests_per_sec = if elapsed.as_secs_f64() > 0.0 {
        total as f64 / elapsed.as_secs_f64()
    } else {
        0.0
    };

    println!("  ├─────────────────────────────────────────────────────────────────┤");
    if skipped > 0 {
        println!(
            "  │ {}: {} passed, {} failed, {} skipped | {:.2}s ({:.1} tests/sec)",
            mode.cyan().bold(),
            passed.to_string().green(),
            failed.to_string().red(),
            skipped.to_string().yellow(),
            elapsed.as_secs_f64(),
            tests_per_sec
        );
    } else {
        println!(
            "  │ {}: {} passed, {} failed | {:.2}s ({:.1} tests/sec)",
            mode.cyan().bold(),
            passed.to_string().green(),
            failed.to_string().red(),
            elapsed.as_secs_f64(),
            tests_per_sec
        );
    }
    println!("  └─────────────────────────────────────────────────────────────────┘");
}

/// Runs in TUI mode.
fn run_tui_mode(runner: &TestRunner) -> ExitCode {
    match tui::run(runner) {
        Ok(success) => {
            if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("{} TUI error: {e}", "ERROR:".red().bold());
            ExitCode::FAILURE
        }
    }
}
