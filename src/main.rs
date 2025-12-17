//! forge-e2e: E2E validation tool for forge-demo.
//!
//! Validates forge-demo calculations against Gnumeric.
//! Default: TUI mode | --all: verbose headless mode

mod engine;
mod runner;
mod tui;
mod types;

use std::path::PathBuf;
use std::process::ExitCode;

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
fn run_all_mode(runner: &TestRunner) -> ExitCode {
    println!();
    println!("{}", "═".repeat(70).cyan());
    println!("{}", "  forge-e2e: E2E Validation Suite".cyan().bold());
    println!("{}", "═".repeat(70).cyan());
    println!();

    let results = runner.run_all();

    let mut passed = 0;
    let mut failed = 0;

    for result in &results {
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
            }
        }
    }

    println!();
    println!("{}", "═".repeat(70).cyan());
    println!(
        "  Results: {} {}, {} {}",
        passed.to_string().green().bold(),
        "passed".green(),
        failed.to_string().red().bold(),
        "failed".red()
    );
    println!("{}", "═".repeat(70).cyan());
    println!();

    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
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
