//! Spreadsheet engine detection and conversion
//!
//! Detects available spreadsheet engines (Gnumeric/LibreOffice) and
//! provides XLSX to CSV conversion with formula recalculation.

use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

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
                // LibreOffice requires UNO API to recalculate formulas in headless mode.
                // We start LO with socket listener, then use Python-UNO to recalc and save.
                self.libreoffice_convert_with_recalc(path, xlsx_path, &csv_path)?;
            }
        }

        if csv_path.exists() {
            Ok(csv_path)
        } else {
            Err(format!("CSV file not created: {:?}", csv_path))
        }
    }

    /// Convert XLSX to CSV using LibreOffice with formula recalculation via UNO API
    fn libreoffice_convert_with_recalc(
        &self,
        lo_path: &Path,
        xlsx_path: &Path,
        csv_path: &Path,
    ) -> Result<(), String> {
        const LO_PORT: u16 = 2002;

        // Start LibreOffice in listening mode
        let mut lo_proc = Command::new(lo_path)
            .args([
                "--headless",
                &format!(
                    "--accept=socket,host=localhost,port={};urp;StarOffice.ServiceManager",
                    LO_PORT
                ),
                "--nofirststartwizard",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start LibreOffice: {}", e))?;

        // Wait for LibreOffice to start listening
        let connected = Self::wait_for_port(LO_PORT, Duration::from_secs(15));
        if !connected {
            let _ = lo_proc.kill();
            return Err("LibreOffice failed to start listening".to_string());
        }

        // Small delay for full initialization
        thread::sleep(Duration::from_millis(500));

        // Run Python script to recalculate and save
        let result = self.run_uno_convert(xlsx_path, csv_path);

        // Terminate LibreOffice
        let _ = lo_proc.kill();
        let _ = lo_proc.wait();

        result
    }

    /// Wait for a TCP port to become available
    fn wait_for_port(port: u16, timeout: Duration) -> bool {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                return true;
            }
            thread::sleep(Duration::from_millis(200));
        }
        false
    }

    /// Run Python-UNO script to recalculate and convert
    fn run_uno_convert(&self, xlsx_path: &Path, csv_path: &Path) -> Result<(), String> {
        let python_script = r#"
import sys, os
sys.path.insert(0, '/usr/lib/libreoffice/program')
import uno
from com.sun.star.beans import PropertyValue

input_url = "file://" + sys.argv[1]
output_url = "file://" + sys.argv[2]

localContext = uno.getComponentContext()
resolver = localContext.ServiceManager.createInstanceWithContext(
    "com.sun.star.bridge.UnoUrlResolver", localContext)
ctx = resolver.resolve(
    "uno:socket,host=localhost,port=2002;urp;StarOffice.ComponentContext")
smgr = ctx.ServiceManager
desktop = smgr.createInstanceWithContext("com.sun.star.frame.Desktop", ctx)

load_props = (PropertyValue("Hidden", 0, True, 0),)
doc = desktop.loadComponentFromURL(input_url, "_blank", 0, load_props)
if doc is None:
    sys.exit(1)
try:
    if hasattr(doc, 'calculateAll'):
        doc.calculateAll()
    if hasattr(doc, 'getSheets'):
        sheets = doc.getSheets()
        for i in range(sheets.getCount()):
            sheet = sheets.getByIndex(i)
            if hasattr(sheet, 'calculateAll'):
                sheet.calculateAll()
    save_props = (
        PropertyValue("FilterName", 0, "Text - txt - csv (StarCalc)", 0),
        PropertyValue("FilterOptions", 0, "44,34,76,1", 0),
        PropertyValue("Overwrite", 0, True, 0),
    )
    doc.storeToURL(output_url, save_props)
finally:
    doc.close(True)
"#;

        // Write script to temp file
        let temp_dir =
            tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let script_path = temp_dir.path().join("lo_convert.py");
        let mut script_file = std::fs::File::create(&script_path)
            .map_err(|e| format!("Failed to create script file: {}", e))?;
        script_file
            .write_all(python_script.as_bytes())
            .map_err(|e| format!("Failed to write script: {}", e))?;

        let xlsx_abs = xlsx_path
            .canonicalize()
            .map_err(|e| format!("Failed to get absolute path for xlsx: {}", e))?;
        let csv_abs = csv_path
            .parent()
            .ok_or("Invalid csv path")?
            .canonicalize()
            .map_err(|e| format!("Failed to get absolute path for csv dir: {}", e))?
            .join(csv_path.file_name().ok_or("Invalid csv filename")?);

        let output = Command::new("python3")
            .env("PYTHONPATH", "/usr/lib/libreoffice/program")
            .arg(&script_path)
            .arg(xlsx_abs.to_string_lossy().as_ref())
            .arg(csv_abs.to_string_lossy().as_ref())
            .output()
            .map_err(|e| format!("Failed to run Python: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "Python UNO conversion failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
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
