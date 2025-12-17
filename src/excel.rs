//! Excel helpers for E2E testing forge-demo's import/export commands.
//!
//! Provides:
//! - Create test Excel files with data and formulas
//! - Read Excel files to verify exports
//! - Compare Excel contents for round-trip validation

// Allow unused code - these helpers are only used in tests
#![allow(dead_code)]

use std::path::Path;

use calamine::{open_workbook, Data, Reader, Xlsx};
use rust_xlsxwriter::{Formula, Workbook, XlsxError};

// ─────────────────────────────────────────────────────────────────────────────
// Test Excel Creation
// ─────────────────────────────────────────────────────────────────────────────

/// Creates a test Excel file with scalars for import testing.
///
/// Creates a "Scalars" worksheet with name/value pairs, some with formulas.
pub fn create_test_scalars_xlsx(path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("Scalars")?;

    // Column headers
    sheet.write(0, 0, "Name")?;
    sheet.write(0, 1, "Value")?;

    // Simple values
    sheet.write(1, 0, "revenue")?;
    sheet.write(1, 1, 100_000.0)?;

    sheet.write(2, 0, "costs")?;
    sheet.write(2, 1, 40_000.0)?;

    // Formula
    sheet.write(3, 0, "profit")?;
    sheet.write_formula(3, 1, Formula::new("=B2-B3"))?;

    sheet.write(4, 0, "margin")?;
    sheet.write_formula(4, 1, Formula::new("=B4/B2"))?;

    workbook.save(path)?;
    Ok(())
}

/// Creates a test Excel file with a data table for import testing.
///
/// Creates a worksheet with columnar data (like a P&L statement).
pub fn create_test_table_xlsx(path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name("QuarterlyData")?;

    // Headers (row 0)
    sheet.write(0, 0, "quarter")?;
    sheet.write(0, 1, "revenue")?;
    sheet.write(0, 2, "costs")?;
    sheet.write(0, 3, "profit")?;

    // Q1
    sheet.write(1, 0, "Q1")?;
    sheet.write(1, 1, 100_000.0)?;
    sheet.write(1, 2, 40_000.0)?;
    sheet.write_formula(1, 3, Formula::new("=B2-C2"))?;

    // Q2
    sheet.write(2, 0, "Q2")?;
    sheet.write(2, 1, 120_000.0)?;
    sheet.write(2, 2, 48_000.0)?;
    sheet.write_formula(2, 3, Formula::new("=B3-C3"))?;

    // Q3
    sheet.write(3, 0, "Q3")?;
    sheet.write(3, 1, 130_000.0)?;
    sheet.write(3, 2, 52_000.0)?;
    sheet.write_formula(3, 3, Formula::new("=B4-C4"))?;

    // Q4
    sheet.write(4, 0, "Q4")?;
    sheet.write(4, 1, 150_000.0)?;
    sheet.write(4, 2, 60_000.0)?;
    sheet.write_formula(4, 3, Formula::new("=B5-C5"))?;

    workbook.save(path)?;
    Ok(())
}

/// Creates a comprehensive test Excel file with multiple sheets.
pub fn create_multi_sheet_xlsx(path: &Path) -> Result<(), XlsxError> {
    let mut workbook = Workbook::new();

    // Scalars sheet
    let scalars = workbook.add_worksheet();
    scalars.set_name("Scalars")?;
    scalars.write(0, 0, "Name")?;
    scalars.write(0, 1, "Value")?;
    scalars.write(1, 0, "tax_rate")?;
    scalars.write(1, 1, 0.25)?;
    scalars.write(2, 0, "growth_rate")?;
    scalars.write(2, 1, 0.1)?;

    // Data sheet
    let data = workbook.add_worksheet();
    data.set_name("Revenue")?;
    data.write(0, 0, "month")?;
    data.write(0, 1, "amount")?;
    data.write(1, 0, "Jan")?;
    data.write(1, 1, 10000.0)?;
    data.write(2, 0, "Feb")?;
    data.write(2, 1, 12000.0)?;
    data.write(3, 0, "Mar")?;
    data.write(3, 1, 15000.0)?;

    workbook.save(path)?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Excel Reading
// ─────────────────────────────────────────────────────────────────────────────

/// Cell value from an Excel file.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    /// Empty cell.
    Empty,
    /// Numeric value.
    Number(f64),
    /// String value.
    Text(String),
    /// Boolean value.
    Bool(bool),
    /// Error value.
    Error(String),
}

impl CellValue {
    /// Returns the numeric value if this is a number.
    pub const fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Returns the string value if this is text.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }
}

impl From<&Data> for CellValue {
    #[allow(clippy::cast_precision_loss)]
    fn from(dt: &Data) -> Self {
        match dt {
            Data::Empty => Self::Empty,
            Data::Int(i) => Self::Number(*i as f64),
            Data::Float(f) => Self::Number(*f),
            Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => Self::Text(s.clone()),
            Data::Bool(b) => Self::Bool(*b),
            Data::Error(e) => Self::Error(format!("{e:?}")),
            Data::DateTime(dt) => Self::Number(dt.as_f64()),
        }
    }
}

/// Sheet data from an Excel file.
pub type SheetData = Vec<(String, Vec<Vec<CellValue>>)>;

/// Reads an Excel file and returns sheet data.
pub fn read_xlsx(path: &Path) -> Result<SheetData, String> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("Failed to open Excel file: {e}"))?;

    let sheet_names = workbook.sheet_names();
    let mut sheets = Vec::new();

    for name in sheet_names {
        let range = workbook
            .worksheet_range(&name)
            .map_err(|e| format!("Failed to read sheet {name}: {e}"))?;

        let mut rows = Vec::new();
        for row in range.rows() {
            let cells: Vec<CellValue> = row.iter().map(CellValue::from).collect();
            rows.push(cells);
        }
        sheets.push((name, rows));
    }

    Ok(sheets)
}

/// Gets the sheet names from an Excel file.
pub fn get_sheet_names(path: &Path) -> Result<Vec<String>, String> {
    let workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("Failed to open Excel file: {e}"))?;
    Ok(workbook.sheet_names())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Data;

    // ─────────────────────────────────────────────────────────────────────────
    // CellValue Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn cell_value_as_number() {
        let num = CellValue::Number(42.0);
        assert_eq!(num.as_number(), Some(42.0));

        let text = CellValue::Text("hello".to_string());
        assert_eq!(text.as_number(), None);
    }

    #[test]
    fn cell_value_as_text() {
        let text = CellValue::Text("hello".to_string());
        assert_eq!(text.as_text(), Some("hello"));

        let num = CellValue::Number(42.0);
        assert_eq!(num.as_text(), None);
    }

    #[test]
    fn cell_value_from_data_type() {
        assert_eq!(CellValue::from(&Data::Empty), CellValue::Empty);
        assert_eq!(CellValue::from(&Data::Int(42)), CellValue::Number(42.0));
        assert_eq!(CellValue::from(&Data::Float(2.5)), CellValue::Number(2.5));
        assert_eq!(
            CellValue::from(&Data::String("test".to_string())),
            CellValue::Text("test".to_string())
        );
        assert_eq!(CellValue::from(&Data::Bool(true)), CellValue::Bool(true));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Create Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn create_test_scalars_xlsx_creates_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("scalars.xlsx");

        let result = create_test_scalars_xlsx(&path);
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn create_test_table_xlsx_creates_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("table.xlsx");

        let result = create_test_table_xlsx(&path);
        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn create_multi_sheet_xlsx_creates_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("multi.xlsx");

        let result = create_multi_sheet_xlsx(&path);
        assert!(result.is_ok());
        assert!(path.exists());
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Read Tests
    // ─────────────────────────────────────────────────────────────────────────

    #[test]
    fn read_scalars_xlsx() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("scalars.xlsx");

        create_test_scalars_xlsx(&path).unwrap();
        let sheets = read_xlsx(&path).unwrap();

        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].0, "Scalars");

        // Check header row
        let rows = &sheets[0].1;
        assert!(rows.len() >= 5);
        assert_eq!(rows[0][0].as_text(), Some("Name"));
        assert_eq!(rows[0][1].as_text(), Some("Value"));

        // Check data
        assert_eq!(rows[1][0].as_text(), Some("revenue"));
        assert_eq!(rows[1][1].as_number(), Some(100_000.0));
    }

    #[test]
    fn read_table_xlsx() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("table.xlsx");

        create_test_table_xlsx(&path).unwrap();
        let sheets = read_xlsx(&path).unwrap();

        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].0, "QuarterlyData");

        let rows = &sheets[0].1;
        assert!(rows.len() >= 5); // Header + 4 quarters
    }

    #[test]
    fn read_multi_sheet_xlsx() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("multi.xlsx");

        create_multi_sheet_xlsx(&path).unwrap();
        let sheets = read_xlsx(&path).unwrap();

        assert_eq!(sheets.len(), 2);

        let names: Vec<_> = sheets.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"Scalars"));
        assert!(names.contains(&"Revenue"));
    }

    #[test]
    fn get_sheet_names_works() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("multi.xlsx");

        create_multi_sheet_xlsx(&path).unwrap();
        let names = get_sheet_names(&path).unwrap();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Scalars".to_string()));
        assert!(names.contains(&"Revenue".to_string()));
    }

    #[test]
    fn read_nonexistent_file_returns_error() {
        let result = read_xlsx(Path::new("/nonexistent/file.xlsx"));
        assert!(result.is_err());
    }

    #[test]
    fn get_sheet_names_nonexistent_returns_error() {
        let result = get_sheet_names(Path::new("/nonexistent/file.xlsx"));
        assert!(result.is_err());
    }
}
