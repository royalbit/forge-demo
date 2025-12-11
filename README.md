# Forge

**AI hallucinates numbers. Forge doesn't.**

YAML-based financial modeling with Excel formula evaluation. Git-native. AI-friendly. Token-efficient.

---

## The Problem

- **LLMs hallucinate numbers** — even GPT-5 and Claude Opus can't reliably calculate compound interest
- **Excel files cost tokens** — MCP tools add ~12k token overhead; conversion loses formulas
- **CSV loses logic** — flat data only, no formulas, no metadata
- **FP&A platforms cost $50K-$200K/year** — Anaplan, Pigment, Datarails

## The Solution

Write financial models in YAML. Forge evaluates formulas and exports to Excel.

**Why YAML?**
- LLMs are trained on 13M+ YAML files ([The Stack](https://huggingface.co/datasets/bigcode/the-stack))
- Formulas preserved: `"=revenue - costs"` — AI sees logic, not just numbers
- Rich metadata: units, notes, sources
- Git-native: meaningful diffs and version control

See [docs/WHY_YAML.md](docs/WHY_YAML.md) for the full comparison.

```yaml
_forge_version: "1.0.0"

assumptions:
  revenue:
    value: 1000000
    formula: null
  cogs:
    value: 400000
    formula: null
  gross_profit:
    value: null
    formula: "=revenue - cogs"
  gross_margin:
    value: null
    formula: "=gross_profit / revenue"
```

```bash
$ forge-demo calculate model.yaml
revenue:      1,000,000.00
cogs:           400,000.00
gross_profit:   600,000.00
gross_margin:         0.60

$ forge-demo export model.yaml --output model.xlsx
Exported to model.xlsx
```

---

## Download

**Binary releases:** [royalbit.ca/forge](https://royalbit.ca/forge/)

Available for Linux (x86_64, arm64), macOS (Intel + Apple Silicon), and Windows.

---

## Validation

Forge formulas are validated against Gnumeric and LibreOffice Calc using `forge-e2e`.

### Quick Start

```bash
# Download and run forge-e2e (TUI mode)
./run-demo.sh

# Or run in verbose headless mode
./run-demo.sh --all
```

### How It Works

```
1. forge-demo export model.yaml output.xlsx  # Export with formulas
2. ssconvert --recalc output.xlsx output.csv # Gnumeric recalculates
3. Compare Forge values vs spreadsheet values # Exact match required
```

### Example Output

```bash
$ ./bin/forge-e2e --all
Engine: Gnumeric (ssconvert) (gnumeric 1.12.57)

══════════════════════════════════════════════════════════════════════
  forge-e2e: E2E Validation Suite
══════════════════════════════════════════════════════════════════════

  ✓ assumptions.test_abs_positive = 42
  ✓ assumptions.test_abs_negative = 42
  ✓ assumptions.test_sum_basic = 100
  ✓ assumptions.test_if_true = 100
  ...

══════════════════════════════════════════════════════════════════════
  Results: 45 passed, 0 failed
══════════════════════════════════════════════════════════════════════
```

### Requirements

- **Gnumeric** (preferred): `ssconvert --version`
- **LibreOffice** (fallback): `libreoffice --version`

---

## Stats

| Metric | Value |
|--------|-------|
| Version | v7.2.0 - 100% Test Integrity |
| Lines of Code | 28,000 (Rust) |
| Test Coverage | 90% |
| Automated Tests | 1,267 |
| Demo Functions | 48 |
| Enterprise Functions | 160 |

---

## Demo Features (48 Functions)

| Category | Count | Functions |
|----------|-------|-----------|
| Math | 9 | ABS, SQRT, ROUND, ROUNDUP, ROUNDDOWN, FLOOR, CEILING, MOD, POWER |
| Aggregation | 5 | SUM, AVERAGE, MIN, MAX, COUNT |
| Logical | 5 | IF, AND, OR, NOT, IFERROR |
| Text | 9 | CONCAT, LEFT, RIGHT, MID, LEN, UPPER, LOWER, TRIM, REPT |
| Date | 6 | TODAY, DATE, YEAR, MONTH, DAY, DATEDIF |
| Lookup | 3 | INDEX, MATCH, CHOOSE |

See [docs/FUNCTIONS.md](docs/FUNCTIONS.md) for the complete function reference.

---

## Enterprise Edition

The enterprise edition (licensed separately) includes:

- **160 functions** (148 Excel + 6 FP&A that Excel doesn't have)
- **Financial functions** — NPV, IRR, PMT, PV, FV, XNPV, XIRR
- **FP&A functions** — VARIANCE, VARIANCE_PCT, BREAKEVEN_UNITS, BREAKEVEN_REVENUE
- **Statistical functions** — MEDIAN, VAR, STDEV, PERCENTILE
- **Trigonometric functions** — SIN, COS, TAN, ASIN, ACOS, ATAN
- **Information functions** — ISBLANK, ISERROR, ISNUMBER, ISTEXT
- **Advanced schemas** — v1.0.0 + v5.0.0 with rich metadata
- **forge-server** — REST API for integrations
- **forge-mcp** — AI/MCP server for Claude, GPT integration

[Open an issue](https://github.com/royalbit/forge-demo/issues/new) for enterprise inquiries.

---

## Use Cases

- **FP&A teams** replacing Excel with version-controlled models
- **AI/LLM pipelines** needing reliable financial calculations
- **Consultants** building client models with audit trails
- **Developers** integrating financial logic into applications

---

## Security

- **100% local** — no cloud, no telemetry, no data leaves your machine
- **Deterministic** — same input = same output, always
- **Auditable** — YAML is human-readable, git-diffable

---

## Contact

Interested in this research? Have questions? Open an issue.

- [General Inquiry](https://github.com/royalbit/forge/issues/new?template=inquiry.md)
- [Bug Report](https://github.com/royalbit/forge/issues/new?template=bug_report.md)
- [Feature Request](https://github.com/royalbit/forge/issues/new?template=feature_request.md)

---

**R&D project by RoyalBit Inc.** | Montreal, Quebec, Canada
