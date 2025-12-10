//! Common types for forge-e2e

#![allow(dead_code)]

use serde::Deserialize;
use std::collections::HashMap;

/// Test specification file structure
#[derive(Debug, Deserialize)]
pub struct TestSpec {
    #[serde(rename = "_forge_version")]
    pub forge_version: String,

    #[serde(flatten)]
    pub sections: HashMap<String, Section>,
}

/// A section in the test spec (e.g., "assumptions", "projections")
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Section {
    ScalarGroup(HashMap<String, Scalar>),
    Table(HashMap<String, TableColumn>),
}

/// A scalar value with optional formula and expected value
#[derive(Debug, Deserialize)]
pub struct Scalar {
    pub value: Option<f64>,
    pub formula: Option<String>,
    /// Expected value for E2E validation (forge-e2e specific)
    pub expected: Option<f64>,
}

/// A table column (array of values or formula)
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TableColumn {
    Numbers(Vec<f64>),
    Strings(Vec<String>),
    Formula(String),
}

/// Individual test case extracted from spec
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub formula: String,
    pub expected: f64,
}

/// Result of running a test
#[derive(Debug)]
pub enum TestResult {
    Pass {
        name: String,
        formula: String,
        expected: f64,
        actual: f64,
    },
    Fail {
        name: String,
        formula: String,
        expected: f64,
        actual: Option<f64>,
        error: Option<String>,
    },
    Skip {
        name: String,
        reason: String,
    },
}

impl TestResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, TestResult::Pass { .. })
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, TestResult::Fail { .. })
    }

    pub fn name(&self) -> &str {
        match self {
            TestResult::Pass { name, .. } => name,
            TestResult::Fail { name, .. } => name,
            TestResult::Skip { name, .. } => name,
        }
    }
}

/// Extract test cases from a test spec
pub fn extract_test_cases(spec: &TestSpec) -> Vec<TestCase> {
    let mut cases = Vec::new();

    for (section_name, section) in &spec.sections {
        // Skip non-test sections
        if section_name.starts_with('_') || section_name == "scenarios" {
            continue;
        }

        match section {
            Section::ScalarGroup(scalars) => {
                for (name, scalar) in scalars {
                    if let (Some(formula), Some(expected)) = (&scalar.formula, scalar.expected) {
                        cases.push(TestCase {
                            name: format!("{}.{}", section_name, name),
                            formula: formula.clone(),
                            expected,
                        });
                    }
                }
            }
            Section::Table(_) => {
                // Table tests not yet implemented
            }
        }
    }

    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_spec() {
        let yaml = r#"
_forge_version: "1.0.0"
assumptions:
  test_abs:
    value: null
    formula: "=ABS(-42)"
    expected: 42
"#;
        let spec: TestSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.forge_version, "1.0.0");

        let cases = extract_test_cases(&spec);
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name, "assumptions.test_abs");
        assert_eq!(cases[0].expected, 42.0);
    }
}
