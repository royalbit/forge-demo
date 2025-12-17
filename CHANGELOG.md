# Changelog

All notable changes to forge-demo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [2.0.0] - 2025-12-16

### Full 48-Function E2E Coverage

- Trig functions: SIN, COS, TAN, ASIN, ACOS, ATAN
- Log functions: EXP, LN, LOG10
- Math misc: INT, PI, SIGN, TRUNC
- Text case: LOWER, UPPER
- Date arithmetic tests (Gnumeric-compatible)
- 112 E2E tests, all validated against Gnumeric

---

## [1.9.0] - 2025-12-16

### Side-by-Side Comparison

- Toggle comparison mode with 'c' key
- Split view: Expected (Forge) | Actual (Gnumeric)
- Color-coded match highlighting
- 75 unit tests

---

## [1.8.0] - 2025-12-16

### Enterprise Teaser

- Display '+126 functions in Enterprise'
- Demo Mode indicator
- Upgrade panel in TUI

---

## [1.7.0] - 2025-12-16

### Function Coverage Report

- Coverage summary: X/47 functions validated
- Category breakdown (math, text, date, etc.)
- Coverage panel in TUI

---

## [1.6.0] - 2025-12-16

### Performance Benchmarks

- Tests/sec metric in stats panel
- Elapsed time display
- Performance tracking from start to finish

---

## [1.5.0] - 2025-12-16

### Excel Import/Export E2E Tests

- Excel helpers module for E2E testing
- Import tests for forge-demo import command
- Test Excel file creation with calamine/rust_xlsxwriter
- 65 unit tests (4 export tests ignored pending schema fix)

---

## [1.4.0] - 2025-12-16

### TUI Enhancement - Export & Status

- Save results to JSON with s key
- Status message display for user feedback
- 49 unit tests

---

## [1.3.0] - 2025-12-16

### TUI Enhancement - Search & Visual

- Search test names with / key (case-insensitive)
- Color-coded function categories (math=blue, text=yellow, etc.)
- Pass/fail distribution bar in stats panel
- 47 unit tests

---

## [1.2.0] - 2025-12-16

### TUI Enhancement - Navigation & Details

- Scrollable results list with j/k and arrow key navigation
- Tab between panels (results, details, stats)
- Detail pane showing formula + expected + actual for selected test
- Filter view: all / passed / failed (toggle with 1/2/3 keys)
- Side-by-side layout: test list (left) + detail view (right)
- Clippy pedantic + nursery lints enabled
- 40 unit tests

---

## [1.0.2] - 2025-12-10

### Schema Integrity Fix

- Verified examples/*.yaml are valid v1.0.0 (scalar-only)
- forge v7.2.6: ADR-014 schema versioning, JSON schema validation
- forge v7.2.7: Upgrade command gated to enterprise only
- Git history squashed (4 commits)

---

## [1.0.0] - 2025-12-10

### E2E Validation TUI (forge-e2e)

- Rust CLI with ratatui TUI
- Validates against Gnumeric (ssconvert --recalc)
- 48 demo functions, v1.0.0 schema (scalar only)
- Exact match comparison (no tolerance - financial tool)
- Cross-platform binaries via GitHub Releases

---

## [7.2.1] - 2025-12-11

### Fixed

- README: Corrected test count from 1,267 to 1,258 (verified by `cargo test`)

### Documentation

- Test count verified by running demo build in forge repo (no features)
- 1,250 tests pass, 8 ignored = 1,258 total

---

## [7.2.0] - 2025-12-10

### Initial Public Release

**forge-demo** is an R&D preview of Forge - a deterministic YAML formula calculator.

#### Features
- 48 Excel-compatible functions (demo subset)
- E2E validation against Gnumeric (70 test cases)
- YAML-based formula definitions
- CLI for calculation, export, and validation

#### Stats
- 1,258 automated tests
- 28,000 LOC (Rust)
- 90% test coverage
- 48 demo functions (enterprise has 160)

#### E2E Validation
- 70 formulas validated against Gnumeric
- Categories: math, aggregation, date, logical, lookup, text
- `forge-e2e --all` runs full validation suite

---

*Built with [RoyalBit Asimov](https://github.com/royalbit/asimov)*
