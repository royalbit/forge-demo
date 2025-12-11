# Forge Demo Functions (48 Total)

The demo version includes 48 Excel-compatible functions for basic financial modeling.

## Math Functions (9)

| Function | Description | Example |
|----------|-------------|---------|
| `ABS(x)` | Absolute value | `=ABS(-5)` → 5 |
| `SQRT(x)` | Square root | `=SQRT(144)` → 12 |
| `ROUND(x, d)` | Round to d decimals | `=ROUND(3.456, 2)` → 3.46 |
| `ROUNDUP(x, d)` | Round up | `=ROUNDUP(3.421, 2)` → 3.43 |
| `ROUNDDOWN(x, d)` | Round down | `=ROUNDDOWN(3.456, 2)` → 3.45 |
| `FLOOR(x, s)` | Round down to multiple | `=FLOOR(2.7, 1)` → 2 |
| `CEILING(x, s)` | Round up to multiple | `=CEILING(2.3, 1)` → 3 |
| `MOD(x, y)` | Remainder | `=MOD(10, 3)` → 1 |
| `POWER(x, y)` | x to the power y | `=POWER(2, 10)` → 1024 |

## Aggregation Functions (5)

| Function | Description | Example |
|----------|-------------|---------|
| `SUM(...)` | Sum of values | `=SUM(1, 2, 3)` → 6 |
| `AVERAGE(...)` | Mean | `=AVERAGE(1, 2, 3)` → 2 |
| `MIN(...)` | Minimum | `=MIN(5, 3, 8)` → 3 |
| `MAX(...)` | Maximum | `=MAX(5, 3, 8)` → 8 |
| `COUNT(...)` | Count numbers | `=COUNT(1, 2, 3)` → 3 |

## Logical Functions (5)

| Function | Description | Example |
|----------|-------------|---------|
| `IF(cond, t, f)` | Conditional | `=IF(a>0, 1, 0)` |
| `AND(...)` | All true | `=AND(a>0, b>0)` |
| `OR(...)` | Any true | `=OR(a>0, b>0)` |
| `NOT(x)` | Negate | `=NOT(a>0)` |
| `IFERROR(x, alt)` | Error handler | `=IFERROR(a/b, 0)` |

## Text Functions (9)

| Function | Description | Example |
|----------|-------------|---------|
| `CONCAT(...)` | Join strings | `=CONCAT("a", "b")` → "ab" |
| `LEFT(s, n)` | Left n chars | `=LEFT("Hello", 2)` → "He" |
| `RIGHT(s, n)` | Right n chars | `=RIGHT("Hello", 2)` → "lo" |
| `MID(s, start, len)` | Extract substring | `=MID("Hello", 2, 3)` → "ell" |
| `LEN(s)` | Length | `=LEN("Hello")` → 5 |
| `UPPER(s)` | Uppercase | `=UPPER("hello")` → "HELLO" |
| `LOWER(s)` | Lowercase | `=LOWER("HELLO")` → "hello" |
| `TRIM(s)` | Remove spaces | `=TRIM("  hi  ")` → "hi" |
| `REPT(s, n)` | Repeat string | `=REPT("*", 3)` → "***" |

## Date Functions (6)

| Function | Description | Example |
|----------|-------------|---------|
| `TODAY()` | Current date | `=TODAY()` |
| `DATE(y, m, d)` | Create date | `=DATE(2025, 12, 9)` |
| `YEAR(date)` | Extract year | `=YEAR(date)` → 2025 |
| `MONTH(date)` | Extract month | `=MONTH(date)` → 12 |
| `DAY(date)` | Extract day | `=DAY(date)` → 9 |
| `DATEDIF(s, e, u)` | Date difference | `=DATEDIF(start, end, "d")` |

## Lookup Functions (3)

| Function | Description | Example |
|----------|-------------|---------|
| `INDEX(range, row)` | Get value by position | `=INDEX({10,20,30}, 2)` → 20 |
| `MATCH(val, range, type)` | Find position | `=MATCH(20, {10,20,30}, 0)` → 2 |
| `CHOOSE(n, ...)` | Pick nth value | `=CHOOSE(2, 10, 20, 30)` → 20 |

> **Note**: In v1.0.0 (scalar-only), use inline values. Table column references like `data.values` require v5.0.0.

---

## Enterprise Version (160 Functions)

The enterprise version includes additional functions for professional financial modeling:

### Additional Math Functions
- `EXP`, `LN`, `LOG`, `LOG10`, `INT`, `SIGN`, `TRUNC`, `PI`

### Trigonometric Functions
- `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`
- `SINH`, `COSH`, `TANH`, `RADIANS`, `DEGREES`

### Financial Functions
- `PMT`, `PV`, `FV`, `NPV`, `IRR`, `MIRR`
- `XNPV`, `XIRR` (irregular cashflows)
- `NPER`, `RATE`, `SLN`, `DB`, `DDB`

### Statistical Functions
- `MEDIAN`, `VAR`, `STDEV`
- `PERCENTILE`, `QUARTILE`
- `COUNTA`, `PRODUCT`, `CORREL`

### Advanced Date
- `EDATE`, `EOMONTH`, `NOW`
- `NETWORKDAYS`, `WORKDAY`
- `YEARFRAC`, `WEEKDAY`, `WEEKNUM`

### Conditional Aggregation
- `SUMIF`, `SUMIFS`, `COUNTIF`, `COUNTIFS`, `AVERAGEIF`, `AVERAGEIFS`
- `MAXIFS`, `MINIFS`

### Advanced Logic
- `LET`, `LAMBDA`, `SWITCH`, `IFS`
- `XOR`, `IFNA`, `TRUE`, `FALSE`

### Lookup Functions
- `XLOOKUP`, `VLOOKUP`, `HLOOKUP`
- `OFFSET`, `INDIRECT`, `ROW`, `COLUMN`, `ROWS`, `COLUMNS`

### Array Functions
- `UNIQUE`, `FILTER`, `SORT`, `SEQUENCE`, `RANDARRAY`

### Information Functions
- `ISBLANK`, `ISERROR`, `ISNA`, `ISNUMBER`, `ISTEXT`
- `ISLOGICAL`, `TYPE`, `ERROR.TYPE`

### Forge-Native FP&A Functions
- `VARIANCE`, `VARIANCE_PCT`, `VARIANCE_STATUS`
- `BREAKEVEN_UNITS`, `BREAKEVEN_REVENUE`
- `SCENARIO`

### Servers
- `forge-server` — REST API for integrations
- `forge-mcp` — AI/MCP server for Claude, GPT

[Open an issue](https://github.com/royalbit/forge/issues/new?template=inquiry.md) for enterprise licensing inquiries.
