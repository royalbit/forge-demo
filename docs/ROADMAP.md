# Forge Roadmap

This document outlines what's available and in development for Forge.

## Current: Demo Version (v7.2.0)

The demo version is available now with:

- 47 Excel functions
- v1.0.0 schema (scalars only - no arrays/tables)
- Excel export
- E2E validation suite (1,267 tests passing)
- 100% test integrity (15-agent parallel execution)

## Released: Enterprise Version (v7.2.0)

### Demo vs Enterprise

| Feature | Demo | Enterprise |
|---------|------|------------|
| Functions | 47 basic | 159 (153 Excel + 6 FP&A) |
| Schema | v1.0.0 only | v1.0.0 + v5.0.0 |
| Rich metadata | No | Yes (unit, notes, source) |
| Cross-file includes | No | Yes |
| Variance analysis | No | Yes |
| Break-even functions | No | Yes |
| Advanced logic (LET, LAMBDA) | No | Yes |
| Array functions (UNIQUE, FILTER) | No | Yes |

### v5.0.0 Schema - Enterprise Features (Released)

```yaml
_forge_version: "5.0.0"

# Rich metadata
scalars:
  revenue:
    value: 1000000
    unit: USD
    notes: "FY2025 projected revenue"
    source: "Sales forecast Q3"

# Tables with arrays (v5.0.0 only)
tables:
  monthly:
    month: [1, 2, 3, 4, 5, 6]
    revenue: [100, 110, 121, 133, 146, 161]
    growth: "=(revenue - 100) / 100"
```

### Forge-Specific FP&A Functions

Functions designed for FP&A workflows:

**Variance Analysis**
```yaml
budget_variance: "=VARIANCE(actual, budget)"
variance_pct: "=VARIANCE_PCT(actual, budget)"
status: "=VARIANCE_STATUS(actual, budget, 0.05)"
```

**Break-Even**
```yaml
breakeven_units: "=BREAKEVEN_UNITS(fixed_costs, price, variable_cost)"
breakeven_revenue: "=BREAKEVEN_REVENUE(fixed_costs, cm_ratio)"
```

**Advanced Logic**
```yaml
# LET - Named variables in formulas
result: "=LET(x, revenue, y, costs, x - y)"

# LAMBDA - Reusable functions
margin_calc: "=LAMBDA(rev, cost, (rev - cost) / rev)"

# SWITCH - Multiple conditions
rating: "=SWITCH(score, 90, 'A', 80, 'B', 70, 'C', 'F')"
```

**Cross-File Includes**
```yaml
_includes:
  - assumptions.yaml
  - scenarios/base.yaml

scalars:
  total: "=assumptions.revenue + scenarios.adjustment"
```

## Research Areas

Active areas of exploration:

- **MCP Integration** — Model Context Protocol for AI assistants
- **Real-time Collaboration** — Multi-user editing
- **API Server** — REST API for programmatic access
- **Excel Import** — Convert existing .xlsx to YAML

## Release Velocity

| Version | Date | Highlights |
|---------|------|------------|
| v7.2.0 | 2025-12-10 | 100% Test Integrity, 15-agent parallel execution |
| v7.1.1 | 2025-12-10 | XLSX Roundtrip 100%, formula translator fix |
| v7.1.0 | 2025-12-10 | 100% Real Test Coverage, 7-agent parallel |
| v7.0.2 | 2025-12-09 | FP&A Accuracy Hotfix |
| v7.0.0 | 2025-12-08 | Major release |

## Timeline

This is an R&D project. No specific release dates are committed.

Progress updates are shared via GitHub releases and the project website.

---

**Questions?** [Open an issue](https://github.com/royalbit/forge/issues)
