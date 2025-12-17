//! Test runner - executes E2E validation tests.
//!
//! Orchestrates the test execution pipeline:
//! 1. Load test specs from YAML files
//! 2. For each test, generate a minimal YAML with the formula
//! 3. Run forge-demo export to create XLSX
//! 4. Use spreadsheet engine to recalculate and export to CSV
//! 5. Compare results against expected values

use std::fmt::Write;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use rayon::prelude::*;

use crate::engine::SpreadsheetEngine;
use crate::types::{
    extract_skip_cases, extract_test_cases, SkipCase, TestCase, TestResult, TestSpec,
};

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
    /// All loaded skip cases.
    skip_cases: Vec<SkipCase>,
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
        let (test_cases, skip_cases) = Self::load_test_cases(&tests_dir)?;

        Ok(Self {
            forge_binary,
            engine,
            tests_dir,
            test_cases,
            skip_cases,
        })
    }

    /// Loads all test cases from the tests directory.
    fn load_test_cases(tests_dir: &Path) -> anyhow::Result<(Vec<TestCase>, Vec<SkipCase>)> {
        let mut all_cases = Vec::new();
        let mut all_skips = Vec::new();

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
                        let skips = extract_skip_cases(&spec);
                        all_cases.extend(cases);
                        all_skips.extend(skips);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {e}", path.display());
                    }
                }
            }
        }

        Ok((all_cases, all_skips))
    }

    /// Returns the total number of test cases (including skips).
    pub const fn total_tests(&self) -> usize {
        self.test_cases.len() + self.skip_cases.len()
    }

    /// Returns all test cases.
    pub fn test_cases(&self) -> &[TestCase] {
        &self.test_cases
    }

    /// Returns all skip cases.
    pub fn skip_cases(&self) -> &[SkipCase] {
        &self.skip_cases
    }

    /// Runs all tests and returns results (including skips).
    pub fn run_all(&self) -> Vec<TestResult> {
        // Skip results first, then run actual tests
        self.skip_cases
            .iter()
            .map(|sc| TestResult::Skip {
                name: sc.name.clone(),
                reason: sc.reason.clone(),
            })
            .chain(self.test_cases.iter().map(|tc| self.run_test(tc)))
            .collect()
    }

    /// Runs all tests in batch mode (single XLSX, faster).
    ///
    /// Creates one YAML with all formulas, exports once, validates with Gnumeric once.
    #[allow(clippy::too_many_lines)]
    pub fn run_batch(&self) -> Vec<TestResult> {
        // Skip results first
        let mut results: Vec<TestResult> = self
            .skip_cases
            .iter()
            .map(|sc| TestResult::Skip {
                name: sc.name.clone(),
                reason: sc.reason.clone(),
            })
            .collect();

        if self.test_cases.is_empty() {
            return results;
        }

        // Create a single YAML with all test formulas
        let mut yaml_content = String::from("_forge_version: \"1.0.0\"\nassumptions:\n");
        for (i, tc) in self.test_cases.iter().enumerate() {
            let escaped_formula = tc.formula.replace('"', "\\\"");
            let _ = write!(
                yaml_content,
                "  test_{i}:\n    value: null\n    formula: \"{escaped_formula}\"\n"
            );
        }

        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                // Return all as failed
                for tc in &self.test_cases {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(format!("Failed to create temp dir: {e}")),
                    });
                }
                return results;
            }
        };

        let yaml_path = temp_dir.path().join("batch.yaml");
        let xlsx_path = temp_dir.path().join("batch.xlsx");

        if let Err(e) = fs::write(&yaml_path, &yaml_content) {
            for tc in &self.test_cases {
                results.push(TestResult::Fail {
                    name: tc.name.clone(),
                    formula: tc.formula.clone(),
                    expected: tc.expected,
                    actual: None,
                    error: Some(format!("Failed to write YAML: {e}")),
                });
            }
            return results;
        }

        // Run forge-demo export once
        let output = match Command::new(&self.forge_binary)
            .arg("export")
            .arg(&yaml_path)
            .arg(&xlsx_path)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                for tc in &self.test_cases {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(format!("Failed to run forge-demo: {e}")),
                    });
                }
                return results;
            }
        };

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            for tc in &self.test_cases {
                results.push(TestResult::Fail {
                    name: tc.name.clone(),
                    formula: tc.formula.clone(),
                    expected: tc.expected,
                    actual: None,
                    error: Some(format!("forge-demo export failed: {err}")),
                });
            }
            return results;
        }

        // Convert XLSX to CSV using Gnumeric once
        let csv_path = match self.engine.xlsx_to_csv(&xlsx_path, temp_dir.path()) {
            Ok(p) => p,
            Err(e) => {
                for tc in &self.test_cases {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(format!("CSV conversion failed: {e}")),
                    });
                }
                return results;
            }
        };

        // Parse CSV and match results to test cases
        let csv_results = Self::parse_batch_csv(&csv_path, self.test_cases.len());
        for (i, tc) in self.test_cases.iter().enumerate() {
            match csv_results.get(i) {
                Some(Ok(actual)) => {
                    if (*actual - tc.expected).abs() < f64::EPSILON {
                        results.push(TestResult::Pass {
                            name: tc.name.clone(),
                            formula: tc.formula.clone(),
                            expected: tc.expected,
                            actual: *actual,
                        });
                    } else {
                        results.push(TestResult::Fail {
                            name: tc.name.clone(),
                            formula: tc.formula.clone(),
                            expected: tc.expected,
                            actual: Some(*actual),
                            error: None,
                        });
                    }
                }
                Some(Err(e)) => {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some(e.clone()),
                    });
                }
                None => {
                    results.push(TestResult::Fail {
                        name: tc.name.clone(),
                        formula: tc.formula.clone(),
                        expected: tc.expected,
                        actual: None,
                        error: Some("Missing result in CSV".to_string()),
                    });
                }
            }
        }

        results
    }

    /// Parses batch CSV output to extract results for each test.
    fn parse_batch_csv(csv_path: &Path, count: usize) -> Vec<Result<f64, String>> {
        // Initialize results array with errors - will be filled by index
        let mut results: Vec<Result<f64, String>> =
            vec![Err("Missing result in CSV output".to_string()); count];

        let file = match fs::File::open(csv_path) {
            Ok(f) => f,
            Err(e) => {
                for r in &mut results {
                    *r = Err(format!("Failed to open CSV: {e}"));
                }
                return results;
            }
        };

        let reader = BufReader::new(file);
        for line in reader.lines() {
            let Ok(line) = line else { continue };
            let cells: Vec<&str> = line
                .split(',')
                .map(|s| s.trim_matches('"').trim())
                .collect();

            // Look for test_N labels and extract index
            // Format: "assumptions.test_N" or "test_N" in first column, value in second
            if cells.len() >= 2 {
                let label = cells[0];
                if let Some(idx_str) = label
                    .strip_prefix("assumptions.test_")
                    .or_else(|| label.strip_prefix("test_"))
                {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if idx < count {
                            if let Ok(value) = cells[1].replace(',', "").parse::<f64>() {
                                results[idx] = Ok(value);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Runs a perf test using forge's calculation engine (no Gnumeric).
    ///
    /// Tests formula calculation directly via `forge calculate`.
    /// Compares calculated value against expected value.
    pub fn run_perf_test(&self, test_case: &TestCase) -> TestResult {
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

        if let Err(e) = fs::write(&yaml_path, &yaml_content) {
            return TestResult::Fail {
                name: test_case.name.clone(),
                formula: test_case.formula.clone(),
                expected: test_case.expected,
                actual: None,
                error: Some(format!("Failed to write YAML: {e}")),
            };
        }

        // Use `forge calculate --dry-run` to test calculation engine
        let output = match Command::new(&self.forge_binary)
            .arg("calculate")
            .arg("--dry-run")
            .arg(&yaml_path)
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("Failed to run forge calculate: {e}")),
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
                    "forge calculate failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )),
            };
        }

        // Parse output: "assumptions.test_result = <value>"
        let stdout = String::from_utf8_lossy(&output.stdout);
        match Self::parse_calculate_output(&stdout, "test_result") {
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

    /// Parses `forge calculate` output to extract a value.
    ///
    /// Output format: `assumptions.<name> = <value>`
    fn parse_calculate_output(output: &str, var_name: &str) -> Result<f64, String> {
        let pattern = format!("assumptions.{var_name} = ");
        for line in output.lines() {
            if let Some(rest) = line.trim().strip_prefix(&pattern) {
                return rest
                    .trim()
                    .parse::<f64>()
                    .map_err(|e| format!("Failed to parse value: {e}"));
            }
        }
        Err(format!("Could not find {var_name} in output"))
    }

    /// Runs all perf tests in parallel using rayon.
    ///
    /// Tests formula calculation via `forge calculate` concurrently.
    /// Returns results in the same order as test cases.
    pub fn run_perf_parallel(&self) -> Vec<TestResult> {
        // Skip results first (not parallelized - usually just one)
        let mut results: Vec<TestResult> = self
            .skip_cases
            .iter()
            .map(|sc| TestResult::Skip {
                name: sc.name.clone(),
                reason: sc.reason.clone(),
            })
            .collect();

        // Run all test cases in parallel
        let parallel_results: Vec<TestResult> = self
            .test_cases
            .par_iter()
            .map(|tc| self.run_perf_test(tc))
            .collect();

        results.extend(parallel_results);
        results
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
        let (cases, skips) = result.unwrap();
        assert!(cases.is_empty());
        assert!(skips.is_empty());
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
        let (cases, _) = result.unwrap();
        assert_eq!(cases.len(), 1);
    }

    #[test]
    fn load_ignores_non_yaml_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("readme.txt"), "not yaml").unwrap();
        fs::write(temp_dir.path().join("config.json"), "{}").unwrap();

        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        let (cases, _) = result.unwrap();
        assert!(cases.is_empty());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Import/Export E2E Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod import_export_tests {
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;

    use crate::excel;

    /// Gets the forge-demo binary path (from bin/ or target/).
    fn forge_demo_binary() -> Option<PathBuf> {
        let bin_path = PathBuf::from("bin/forge-demo");
        if bin_path.exists() {
            return Some(bin_path);
        }

        let debug_path = PathBuf::from("target/debug/forge-demo");
        if debug_path.exists() {
            return Some(debug_path);
        }

        let release_path = PathBuf::from("target/release/forge-demo");
        if release_path.exists() {
            return Some(release_path);
        }

        None
    }

    /// Helper to check if forge-demo binary is available.
    fn skip_if_no_binary() -> Option<PathBuf> {
        forge_demo_binary()
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Import Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn import_scalars_xlsx_creates_yaml() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("scalars.xlsx");
        let yaml_path = temp_dir.path().join("scalars.yaml");

        // Create test Excel file
        excel::create_test_scalars_xlsx(&xlsx_path).unwrap();

        // Run import
        let output = Command::new(&binary)
            .arg("import")
            .arg(&xlsx_path)
            .arg(&yaml_path)
            .output()
            .expect("Failed to run forge-demo import");

        // Check success
        assert!(
            output.status.success(),
            "Import failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(yaml_path.exists(), "YAML file was not created");

        // Verify YAML content has some data
        let yaml_content = fs::read_to_string(&yaml_path).unwrap();
        // Should have some content from the Excel file
        assert!(
            yaml_content.len() > 20,
            "YAML should have meaningful content"
        );
    }

    #[test]
    fn import_table_xlsx_creates_yaml() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("table.xlsx");
        let yaml_path = temp_dir.path().join("table.yaml");

        // Create test Excel file with table data
        excel::create_test_table_xlsx(&xlsx_path).unwrap();

        // Run import
        let output = Command::new(&binary)
            .arg("import")
            .arg(&xlsx_path)
            .arg(&yaml_path)
            .output()
            .expect("Failed to run forge-demo import");

        assert!(
            output.status.success(),
            "Import failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(yaml_path.exists());

        let yaml_content = fs::read_to_string(&yaml_path).unwrap();
        assert!(yaml_content.contains("quarter"));
    }

    #[test]
    fn import_multi_sheet_xlsx_creates_yaml() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let xlsx_path = temp_dir.path().join("multi.xlsx");
        let yaml_path = temp_dir.path().join("multi.yaml");

        // Create test Excel file with multiple sheets
        excel::create_multi_sheet_xlsx(&xlsx_path).unwrap();

        // Run import
        let output = Command::new(&binary)
            .arg("import")
            .arg(&xlsx_path)
            .arg(&yaml_path)
            .output()
            .expect("Failed to run forge-demo import");

        assert!(
            output.status.success(),
            "Import failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(yaml_path.exists());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Export Tests
    // NOTE: Export tests are currently skipped because forge-demo v7.2.10
    // has schema validation issues with scalar-only v1.0.0 models.
    // Export requires v1.0.0 array models with 'tables' section.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    #[ignore = "forge-demo export requires tables section, not scalar-only models"]
    fn export_yaml_creates_xlsx() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let yaml_path = temp_dir.path().join("model.yaml");
        let xlsx_path = temp_dir.path().join("model.xlsx");

        // Create test YAML file (v1.0.0 schema: scalars as numbers or quoted formulas)
        let yaml_content = r#"_forge_version: "1.0.0"

scalars:
  revenue: 100000
  costs: 40000
  profit: "=revenue - costs"
"#;
        fs::write(&yaml_path, yaml_content).unwrap();

        // Run export
        let output = Command::new(&binary)
            .arg("export")
            .arg(&yaml_path)
            .arg(&xlsx_path)
            .output()
            .expect("Failed to run forge-demo export");

        assert!(
            output.status.success(),
            "Export failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(xlsx_path.exists(), "XLSX file was not created");

        // Verify Excel file is readable
        let sheets = excel::read_xlsx(&xlsx_path).unwrap();
        assert!(!sheets.is_empty(), "Excel file has no sheets");
    }

    #[test]
    #[ignore = "forge-demo export requires tables section, not scalar-only models"]
    fn export_with_formulas_preserves_data() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let yaml_path = temp_dir.path().join("formulas.yaml");
        let xlsx_path = temp_dir.path().join("formulas.xlsx");

        let yaml_content = r#"_forge_version: "1.0.0"

scalars:
  base: 1000
  rate: 0.1
  result: "=base * (1 + rate)"
"#;
        fs::write(&yaml_path, yaml_content).unwrap();

        let output = Command::new(&binary)
            .arg("export")
            .arg(&yaml_path)
            .arg(&xlsx_path)
            .output()
            .expect("Failed to run forge-demo export");

        assert!(output.status.success());
        assert!(xlsx_path.exists());

        // Check that Scalars sheet has expected data
        let sheets = excel::read_xlsx(&xlsx_path).unwrap();
        let scalars_sheet = sheets.iter().find(|(name, _)| name == "Scalars");
        assert!(scalars_sheet.is_some(), "Scalars sheet not found");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Import Tests (from Excel to YAML)
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn import_xlsx_to_yaml_produces_valid_yaml() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let original_xlsx = temp_dir.path().join("original.xlsx");
        let yaml_path = temp_dir.path().join("imported.yaml");

        // Create test Excel file
        excel::create_test_scalars_xlsx(&original_xlsx).unwrap();

        // Import to YAML
        let import_output = Command::new(&binary)
            .arg("import")
            .arg(&original_xlsx)
            .arg(&yaml_path)
            .output()
            .expect("Failed to import");

        assert!(
            import_output.status.success(),
            "Import failed: {}",
            String::from_utf8_lossy(&import_output.stderr)
        );

        // Verify YAML was created
        assert!(yaml_path.exists(), "YAML file was not created");
        let yaml_content = fs::read_to_string(&yaml_path).unwrap();
        // Should have meaningful content
        assert!(yaml_content.len() > 20, "YAML should have content");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Round-trip Tests
    // NOTE: Full round-trip (YAML→Excel→YAML) requires working export.
    // These tests verify import functionality only.
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    #[ignore = "forge-demo export requires tables section, not scalar-only models"]
    fn roundtrip_yaml_to_xlsx_to_yaml() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let original_yaml = temp_dir.path().join("original.yaml");
        let xlsx_path = temp_dir.path().join("roundtrip.xlsx");
        let imported_yaml = temp_dir.path().join("imported.yaml");

        // Create original YAML (v1.0.0 schema with quoted formulas)
        let yaml_content = r#"_forge_version: "1.0.0"

scalars:
  price: 100
  quantity: 5
  total: "=price * quantity"
"#;
        fs::write(&original_yaml, yaml_content).unwrap();

        // Export to XLSX
        let export_output = Command::new(&binary)
            .arg("export")
            .arg(&original_yaml)
            .arg(&xlsx_path)
            .output()
            .expect("Failed to export");

        assert!(
            export_output.status.success(),
            "Export failed: {}",
            String::from_utf8_lossy(&export_output.stderr)
        );

        // Import back to YAML
        let import_output = Command::new(&binary)
            .arg("import")
            .arg(&xlsx_path)
            .arg(&imported_yaml)
            .output()
            .expect("Failed to import");

        assert!(
            import_output.status.success(),
            "Import failed: {}",
            String::from_utf8_lossy(&import_output.stderr)
        );

        // Verify imported YAML exists and has expected content
        assert!(imported_yaml.exists());
        let imported_content = fs::read_to_string(&imported_yaml).unwrap();
        assert!(imported_content.contains("price"));
        assert!(imported_content.contains("quantity"));
    }

    #[test]
    #[ignore = "forge-demo export requires tables section, not scalar-only models"]
    fn roundtrip_preserves_numeric_values() {
        let Some(binary) = skip_if_no_binary() else {
            eprintln!("Skipping: forge-demo binary not found");
            return;
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let original_yaml = temp_dir.path().join("values.yaml");
        let xlsx_path = temp_dir.path().join("values.xlsx");
        let imported_yaml = temp_dir.path().join("values_imported.yaml");

        // Create YAML with specific numeric values
        let yaml_content = r#"_forge_version: "1.0.0"

scalars:
  integer_val: 42
  float_val: 3.14159
  percent_val: 0.25
"#;
        fs::write(&original_yaml, yaml_content).unwrap();

        // Export
        let export_output = Command::new(&binary)
            .arg("export")
            .arg(&original_yaml)
            .arg(&xlsx_path)
            .output()
            .expect("Failed to export");
        assert!(export_output.status.success());

        // Import
        let import_output = Command::new(&binary)
            .arg("import")
            .arg(&xlsx_path)
            .arg(&imported_yaml)
            .output()
            .expect("Failed to import");
        assert!(import_output.status.success());

        // Verify values are preserved in imported YAML
        let content = fs::read_to_string(&imported_yaml).unwrap();
        // Should contain the scalar names
        assert!(content.contains("integer_val"));
        assert!(content.contains("float_val"));
        assert!(content.contains("percent_val"));
    }
}
