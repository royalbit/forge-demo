//! Spreadsheet engine detection and conversion
//!
//! Detects available spreadsheet engines (Gnumeric/LibreOffice) and
//! provides XLSX to CSV conversion with formula recalculation.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Spreadsheet engine types
pub enum SpreadsheetEngine {
    /// Gnumeric's ssconvert - preferred, properly recalculates formulas
    Gnumeric { path: PathBuf, version: String },
    /// LibreOffice - fallback
    LibreOffice { path: PathBuf, version: String },
}

impl SpreadsheetEngine {
    /// Detect available spreadsheet engine
    /// Prefer ssconvert (gnumeric) as it properly recalculates in headless mode
    pub fn detect() -> Option<Self> {
        // Try ssconvert first (gnumeric) - it properly recalculates
        if let Ok(output) = Command::new("ssconvert").arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stderr).trim().to_string();
                return Some(Self::Gnumeric {
                    path: PathBuf::from("ssconvert"),
                    version,
                });
            }
        }

        // Fallback to LibreOffice - check multiple paths
        let lo_paths = [
            "/usr/bin/soffice",
            "/usr/bin/libreoffice",
            "soffice",
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
            "/snap/bin/libreoffice",
            "libreoffice",
        ];

        for path in lo_paths {
            if let Ok(output) = Command::new(path).arg("--version").output() {
                if output.status.success() {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    return Some(Self::LibreOffice {
                        path: PathBuf::from(path),
                        version,
                    });
                }
            }
        }

        None
    }

    /// Get engine version string
    pub fn version(&self) -> &str {
        match self {
            Self::Gnumeric { version, .. } => version,
            Self::LibreOffice { version, .. } => version,
        }
    }

    /// Get engine name
    pub fn name(&self) -> &str {
        match self {
            Self::Gnumeric { .. } => "Gnumeric (ssconvert)",
            Self::LibreOffice { .. } => "LibreOffice",
        }
    }

    /// Convert XLSX to CSV with formula recalculation
    pub fn xlsx_to_csv(&self, xlsx_path: &Path, output_dir: &Path) -> Result<PathBuf, String> {
        let csv_name = xlsx_path.file_stem().unwrap().to_string_lossy().to_string() + ".csv";
        let csv_path = output_dir.join(&csv_name);

        match self {
            Self::Gnumeric { path, .. } => {
                // ssconvert --recalc properly recalculates formulas
                let output = Command::new(path)
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
            }
            Self::LibreOffice { path, .. } => {
                let output = Command::new(path)
                    .args([
                        "--headless",
                        "--convert-to",
                        "csv:Text - txt - csv (StarCalc):44,34,76,1",
                        "--outdir",
                    ])
                    .arg(output_dir)
                    .arg(xlsx_path)
                    .output()
                    .map_err(|e| format!("Failed to run LibreOffice: {}", e))?;

                if !output.status.success() {
                    return Err(format!(
                        "LibreOffice conversion failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
            }
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
        // This test may skip if no engine is available
        if let Some(engine) = SpreadsheetEngine::detect() {
            assert!(!engine.name().is_empty());
            // Version may be empty if engine outputs to different stream
        }
    }
}
