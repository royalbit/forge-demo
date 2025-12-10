# Forge Schema v1.0.0 (Scalar Only)

The v1.0.0 schema is for **scalar-only** financial models. This is the demo edition.

> **Note**: For tables and arrays, upgrade to v5.0.0 (enterprise edition).

## Structure

```yaml
_forge_version: "1.0.0"

scalars:
  input_value: 1000
  calculated_value: "=input_value * 2"
```

## Scalars

Scalars are single values or formulas. This is the ONLY data type in v1.0.0.

```yaml
scalars:
  # Literal values
  revenue: 1000000
  cost_rate: 0.40

  # Formulas (must start with =)
  costs: "=revenue * cost_rate"
  profit: "=revenue - costs"
  margin: "=profit / revenue"
```

## Nested Scalars

Organize scalars into groups:

```yaml
assumptions:
  tax_rate:
    value: 0.25
    formula: null

  growth_rate:
    value: null
    formula: "=0.05 + risk_premium"
```

## Supported Types

- **Numbers**: `42`, `3.14`, `-100`, `0.25`
- **Formulas**: `"=SUM(a, b, c)"`, `"=IF(x > 0, 1, 0)"`

## NOT Supported in v1.0.0

The following require v5.0.0 (enterprise):

- ❌ Arrays: `[1, 2, 3, 4, 5]`
- ❌ Tables with columns
- ❌ Row-wise formulas
- ❌ Column references like `table.column`

## Complete Example

```yaml
_forge_version: "1.0.0"

scalars:
  # Inputs
  price: 99
  units_sold: 1000
  cost_per_unit: 40
  tax_rate: 0.25

  # Calculations
  revenue: "=price * units_sold"
  total_cost: "=cost_per_unit * units_sold"
  gross_profit: "=revenue - total_cost"
  taxes: "=IF(gross_profit > 0, gross_profit * tax_rate, 0)"
  net_profit: "=gross_profit - taxes"

  # Metrics
  gross_margin: "=gross_profit / revenue"
  net_margin: "=net_profit / revenue"
```

## Upgrade to v5.0.0

To use tables and arrays, change the version:

```yaml
_forge_version: "5.0.0"
# Now you can use tables, arrays, and column references
```

See the enterprise documentation for v5.0.0 features.
