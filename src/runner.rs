//! Test runner - executes E2E validation tests.
//!
//! Orchestrates the test execution pipeline:
//! 1. Load test specs from YAML files
//! 2. For each test, generate a minimal YAML with the formula
//! 3. Run forge-demo export to create XLSX
//! 4. Use spreadsheet engine to recalculate and export to CSV
//! 5. Compare results against expected values

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::SpreadsheetEngine;
use crate::types::{extract_test_cases, TestCase, TestResult, TestSpec};

// ─────────────────────────────────────────────────────────────────────────────
// Test Runner
// ─────────────────────────────────────────────────────────────────────────────

/// Test runner for E2E validation.
///
/// Manages test case loading and execution against the forge-demo binary.
pub struct TestRunner {
    /// Path to the forge-demo binary.
    forge_binary: PathBuf,
    /// Spreadsheet engine for formula recalculation.
    engine: SpreadsheetEngine,
    /// Directory containing test spec files.
    #[allow(dead_code)]
    tests_dir: PathBuf,
    /// All loaded test cases.
    test_cases: Vec<TestCase>,
}

impl TestRunner {
    /// Creates a new test runner.
    ///
    /// Loads all test cases from YAML files in the tests directory.
    pub fn new(
        forge_binary: PathBuf,
        engine: SpreadsheetEngine,
        tests_dir: PathBuf,
    ) -> anyhow::Result<Self> {
        let test_cases = Self::load_test_cases(&tests_dir)?;

        Ok(Self {
            forge_binary,
            engine,
            tests_dir,
            test_cases,
        })
    }

    /// Loads all test cases from the tests directory.
    fn load_test_cases(tests_dir: &Path) -> anyhow::Result<Vec<TestCase>> {
        let mut all_cases = Vec::new();

        if !tests_dir.exists() {
            anyhow::bail!("Tests directory does not exist: {}", tests_dir.display());
        }

        for entry in fs::read_dir(tests_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "yaml") {
                let content = fs::read_to_string(&path)?;
                match serde_yaml_ng::from_str::<TestSpec>(&content) {
                    Ok(spec) => {
                        let cases = extract_test_cases(&spec);
                        all_cases.extend(cases);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {e}", path.display());
                    }
                }
            }
        }

        Ok(all_cases)
    }

    /// Returns the total number of test cases.
    pub const fn total_tests(&self) -> usize {
        self.test_cases.len()
    }

    /// Returns all test cases.
    pub fn test_cases(&self) -> &[TestCase] {
        &self.test_cases
    }

    /// Runs all tests and returns results.
    pub fn run_all(&self) -> Vec<TestResult> {
        self.test_cases.iter().map(|tc| self.run_test(tc)).collect()
    }

    /// Runs a single test case.
    ///
    /// Creates a temporary YAML file with the formula, runs forge-demo export,
    /// converts to CSV using the spreadsheet engine, and compares results.
    pub fn run_test(&self, test_case: &TestCase) -> TestResult {
        // Create a minimal YAML with just this test
        // Escape double quotes in formula for YAML compatibility
        let escaped_formula = test_case.formula.replace('"', "\\\"");
        let yaml_content = format!(
            r#"_forge_version: "1.0.0"
assumptions:
  test_result:
    value: null
    formula: "{escaped_formula}"
"#
        );

        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("Failed to create temp dir: {e}")),
                };
            }
        };

        let yaml_path = temp_dir.path().join("test.yaml");
        let xlsx_path = temp_dir.path().join("test.xlsx");

        // Write YAML
        if let Err(e) = fs::write(&yaml_path, &yaml_content) {
            return TestResult::Fail {
                name: test_case.name.clone(),
                formula: test_case.formula.clone(),
                expected: test_case.expected,
                actual: None,
                error: Some(format!("Failed to write YAML: {e}")),
            };
        }

        // Run forge-demo export
        let output = match Command::new(&self.forge_binary)
            .arg("export")
            .arg(&yaml_path)
            .arg(&xlsx_path)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("Failed to run forge-demo: {e}")),
                };
            }
        };

        if !output.status.success() {
            return TestResult::Fail {
                name: test_case.name.clone(),
                formula: test_case.formula.clone(),
                expected: test_case.expected,
                actual: None,
                error: Some(format!(
                    "forge-demo export failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )),
            };
        }

        // Convert XLSX to CSV using spreadsheet engine
        let csv_path = match self.engine.xlsx_to_csv(&xlsx_path, temp_dir.path()) {
            Ok(p) => p,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("CSV conversion failed: {e}")),
                };
            }
        };

        // Parse CSV and find result
        match Self::find_result_in_csv(&csv_path, test_case.expected) {
            Ok(actual) => {
                if (actual - test_case.expected).abs() < f64::EPSILON {
                    TestResult::Pass {
                        name: test_case.name.clone(),
                        formula: test_case.formula.clone(),
                        expected: test_case.expected,
                        actual,
                    }
                } else {
                    TestResult::Fail {
                        name: test_case.name.clone(),
                        formula: test_case.formula.clone(),
                        expected: test_case.expected,
                        actual: Some(actual),
                        error: None,
                    }
                }
            }
            Err(e) => TestResult::Fail {
                name: test_case.name.clone(),
                formula: test_case.formula.clone(),
                expected: test_case.expected,
                actual: None,
                error: Some(e),
            },
        }
    }

    /// Finds the result value in CSV output.
    ///
    /// Looks for labeled results ("result" or `test_result`) or matches
    /// numeric values against the expected value.
    fn find_result_in_csv(csv_path: &Path, expected: f64) -> Result<f64, String> {
        let file = fs::File::open(csv_path).map_err(|e| format!("Failed to open CSV: {e}"))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {e}"))?;
            // Simple CSV parsing
            let cells: Vec<&str> = line
                .split(',')
                .map(|s| s.trim_matches('"').trim())
                .collect();

            for (i, cell) in cells.iter().enumerate() {
                // Look for "result" or "test_result" label followed by value
                if (*cell == "result" || *cell == "test_result") && i + 1 < cells.len() {
                    if let Ok(value) = cells[i + 1].replace(',', "").parse::<f64>() {
                        return Ok(value);
                    }
                }

                // Also try parsing any numeric value
                if let Ok(value) = cell.replace(',', "").parse::<f64>() {
                    // Check if it matches expected (for simple formulas)
                    if (value - expected).abs() < 0.0001 {
                        return Ok(value);
                    }
                }
            }
        }

        Err("Could not find result in CSV output".to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_empty_dir_returns_empty_cases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn load_nonexistent_dir_returns_error() {
        let result = TestRunner::load_test_cases(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn load_dir_with_yaml_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let yaml_content = r#"
_forge_version: "1.0.0"
assumptions:
  test_one:
    value: null
    formula: "=1+1"
    expected: 2
"#;
        fs::write(temp_dir.path().join("test.yaml"), yaml_content).unwrap();

        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        let cases = result.unwrap();
        assert_eq!(cases.len(), 1);
    }

    #[test]
    fn load_ignores_non_yaml_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("readme.txt"), "not yaml").unwrap();
        fs::write(temp_dir.path().join("config.json"), "{}").unwrap();

        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
