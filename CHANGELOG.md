# Changelog

All notable changes to forge-demo will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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
