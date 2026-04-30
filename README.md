# enelyzer-ingestion-emp-mail

A Rust library that parses **Energy Market Price (EMP)** cockpit emails into
strongly-typed, JSON-serialisable structs — complete with a JSON Schema for
validation.

## What it parses

EMP cockpit emails are weekly market-update HTML emails that contain:

| Section | Contents |
|---------|----------|
| **Email metadata** | From, To, Subject, Date, Message-ID |
| **Report header** | Publication date, market-group title (e.g. "NLMK market update") |
| **News items** | Date, headline, excerpt, "Read More" URL |
| **Statistics table** | Market price rows grouped by commodity section |

The statistics table covers four commodity sections:

* **ELECTRICITY** — BE, IT, FR spot and forward contracts (M+1…M+4, Q+2…Q+4, Y+1)
* **GAS** — Gas BE ZTP, TTF, PEG, IT PSV spot and forward contracts
* **OIL** — Brent EUR/bbl
* **OTHER** — CO2 EUA spot

Each market row carries:

| Field | Type | Description |
|-------|------|-------------|
| `market` | `String` | Instrument name (e.g. `"BE spot"`, `"Gas BE ZTP M+2"`) |
| `unit` | `String` | Price unit (e.g. `"EUR/MWh"`, `"EUR/bbl"`) |
| `prices` | `Vec<Option<f64>>` | Daily closing prices (one per date in `price_dates`) |
| `var_d1_pct` | `f64` | Day-over-day change in percent |
| `var_w1_pct` | `f64` | Week-over-week change in percent |
| `avg_ytd` | `f64` | Year-to-date average |
| `max_ytd` | `f64` | Year-to-date maximum |
| `min_ytd` | `f64` | Year-to-date minimum |
| `trend_d1` | `Trend` | `"up"` / `"down"` / `"flat"` from `var_d1_pct` |
| `trend_w1` | `Trend` | `"up"` / `"down"` / `"flat"` from `var_w1_pct` |

## Installation

```toml
# Cargo.toml
[dependencies]
enelyzer-ingestion-emp-mail = { git = "https://github.com/michallis/enelyzer-ingestion-emp-mail" }
```

## Quick start

```rust
use enelyzer_ingestion_emp_mail::parse_eml;

fn main() {
    let raw = std::fs::read("My_Energy_Cockpit.eml").unwrap();
    let mail = parse_eml(&raw).unwrap();

    println!("Report : {} — {}", mail.report.date, mail.report.title);

    for section in &mail.report.statistics.sections {
        println!("\n[{}]", section.name);
        for row in &section.rows {
            println!(
                "  {:30} {:10}  VAR D-1: {:+.2}%  VAR W-1: {:+.2}%",
                row.market, row.unit, row.var_d1_pct, row.var_w1_pct
            );
        }
    }

    // Emit full JSON
    println!("{}", serde_json::to_string_pretty(&mail).unwrap());
}
```

## CLI example

```bash
# Human-readable summary
cargo run --example parse_email -- My_Energy_Cockpit.eml

# Full JSON output
cargo run --example parse_email -- My_Energy_Cockpit.eml --json

# Print the JSON Schema
cargo run --example parse_email -- --schema
```

## JSON Schema

A hand-written, auto-derivable JSON Schema (draft-07) lives in
[`schema/emp_mail.schema.json`](schema/emp_mail.schema.json).

Use it for validation in any language:

```bash
# Example with ajv-cli
npx ajv validate -s schema/emp_mail.schema.json -d output.json
```

Or generate it programmatically from Rust:

```rust
let schema = enelyzer_ingestion_emp_mail::generate_schema();
println!("{}", serde_json::to_string_pretty(&schema).unwrap());
```

## Running tests

```bash
# Unit + integration tests (no fixture needed)
cargo test

# Test against a real .eml file
EMP_FIXTURE_PATH=/path/to/My_Energy_Cockpit.eml cargo test test_real_file -- --ignored
```

## Architecture

```
src/
├── lib.rs       — public API, re-exports, generate_schema()
├── types.rs     — EmpMail, Report, MarketRow, … (serde + schemars derives)
├── parser.rs    — parse_eml(): email decoding → HTML → structured types
└── error.rs     — ParseError (thiserror)

schema/
└── emp_mail.schema.json   — JSON Schema draft-07

examples/
└── parse_email.rs         — CLI tool: summary / --json / --schema

tests/
└── integration_test.rs    — inline fixture tests + real-file test (--ignored)
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `mailparse` | Parse MIME email headers and decode quoted-printable body |
| `scraper` | CSS-selector HTML parsing (html5ever-based) |
| `serde` + `serde_json` | Serialisation / deserialisation |
| `schemars` | JSON Schema generation from Rust types |
| `thiserror` | Ergonomic error enum |
| `regex` | Percentage-value extraction |
| `once_cell` | Static regex compilation |

## License

MIT
