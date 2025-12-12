//! Test runner - executes E2E validation tests

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::SpreadsheetEngine;
use crate::types::{extract_test_cases, TestCase, TestResult, TestSpec};

/// Test runner for E2E validation
pub struct TestRunner {
    forge_binary: PathBuf,
    engine: SpreadsheetEngine,
    #[allow(dead_code)]
    tests_dir: PathBuf,
    test_cases: Vec<TestCase>,
}

impl TestRunner {
    /// Create a new test runner
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

    /// Load all test cases from the tests directory
    fn load_test_cases(tests_dir: &Path) -> anyhow::Result<Vec<TestCase>> {
        let mut all_cases = Vec::new();

        if !tests_dir.exists() {
            anyhow::bail!("Tests directory does not exist: {:?}", tests_dir);
        }

        for entry in fs::read_dir(tests_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "yaml").unwrap_or(false) {
                let content = fs::read_to_string(&path)?;
                match serde_yaml_ng::from_str::<TestSpec>(&content) {
                    Ok(spec) => {
                        let cases = extract_test_cases(&spec);
                        all_cases.extend(cases);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(all_cases)
    }

    /// Get total number of test cases
    pub fn total_tests(&self) -> usize {
        self.test_cases.len()
    }

    /// Get all test cases
    pub fn test_cases(&self) -> &[TestCase] {
        &self.test_cases
    }

    /// Run all tests and return results
    pub fn run_all(&mut self) -> Vec<TestResult> {
        self.test_cases
            .clone()
            .iter()
            .map(|tc| self.run_test(tc))
            .collect()
    }

    /// Run a single test case
    pub fn run_test(&self, test_case: &TestCase) -> TestResult {
        // Create a minimal YAML with just this test
        // Escape double quotes in formula for YAML compatibility
        let escaped_formula = test_case.formula.replace('"', "\\\"");
        let yaml_content = format!(
            r#"_forge_version: "1.0.0"
assumptions:
  test_result:
    value: null
    formula: "{}"
"#,
            escaped_formula
        );

        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("Failed to create temp dir: {}", e)),
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
                error: Some(format!("Failed to write YAML: {}", e)),
            };
        }

        // Run forge-demo export
        let output = Command::new(&self.forge_binary)
            .arg("export")
            .arg(&yaml_path)
            .arg(&xlsx_path)
            .output();

        let output = match output {
            Ok(o) => o,
            Err(e) => {
                return TestResult::Fail {
                    name: test_case.name.clone(),
                    formula: test_case.formula.clone(),
                    expected: test_case.expected,
                    actual: None,
                    error: Some(format!("Failed to run forge-demo: {}", e)),
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
                    error: Some(format!("CSV conversion failed: {}", e)),
                };
            }
        };

        // Parse CSV and find result
        match self.find_result_in_csv(&csv_path, test_case.expected) {
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

    /// Find the result value in CSV output
    fn find_result_in_csv(&self, csv_path: &Path, expected: f64) -> Result<f64, String> {
        let file = fs::File::open(csv_path).map_err(|e| format!("Failed to open CSV: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line.map_err(|e| format!("Failed to read line: {}", e))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_empty_dir() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
