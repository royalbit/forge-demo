//! Spreadsheet engine detection and conversion
//!
//! Uses Gnumeric's ssconvert for XLSX to CSV conversion with formula recalculation.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Gnumeric spreadsheet engine
pub struct SpreadsheetEngine {
    path: PathBuf,
    version: String,
}

impl SpreadsheetEngine {
    /// Detect Gnumeric (ssconvert) installation
    pub fn detect() -> Option<Self> {
        if let Ok(output) = Command::new("ssconvert").arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Some(Self {
                    path: PathBuf::from("ssconvert"),
                    version,
                });
            }
        }
        None
    }

    /// Get engine version string
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get engine name
    pub fn name(&self) -> &str {
        "Gnumeric (ssconvert)"
    }

    /// Convert XLSX to CSV with formula recalculation
    pub fn xlsx_to_csv(&self, xlsx_path: &Path, output_dir: &Path) -> Result<PathBuf, String> {
        let csv_name = xlsx_path.file_stem().unwrap().to_string_lossy().to_string() + ".csv";
        let csv_path = output_dir.join(&csv_name);

        let output = Command::new(&self.path)
            .arg("--recalc")
            .arg(xlsx_path)
            .arg(&csv_path)
            .output()
            .map_err(|e| format!("Failed to run ssconvert: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "ssconvert failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        if csv_path.exists() {
            Ok(csv_path)
        } else {
            Err(format!("CSV file not created: {:?}", csv_path))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_detection() {
        // This test may skip if Gnumeric is not installed
        if let Some(engine) = SpreadsheetEngine::detect() {
            assert!(!engine.name().is_empty());
        }
    }
}
