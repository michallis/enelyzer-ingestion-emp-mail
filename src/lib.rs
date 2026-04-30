//! # enelyzer-ingestion-emp-mail
//!
//! A Rust library that parses **Energy Market Price (EMP)** cockpit emails
//! into strongly-typed, JSON-serialisable structs.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use enelyzer_ingestion_emp_mail::parse_eml;
//!
//! let raw = std::fs::read("My_Energy_Cockpit.eml").expect("cannot read file");
//! let mail = parse_eml(&raw).expect("parse failed");
//!
//! println!("Report date  : {}", mail.report.date);
//! println!("Report title : {}", mail.report.title);
//! println!("News items   : {}", mail.report.news_items.len());
//!
//! for section in &mail.report.statistics.sections {
//!     println!("\n[{}]", section.name);
//!     for row in &section.rows {
//!         println!("  {:30} {:10}  VAR D-1: {:+.2}%  VAR W-1: {:+.2}%",
//!             row.market, row.unit, row.var_d1_pct, row.var_w1_pct);
//!     }
//! }
//!
//! // Serialise to JSON
//! let json = serde_json::to_string_pretty(&mail).unwrap();
//! println!("{}", json);
//! ```
//!
//! ## JSON Schema
//!
//! A JSON Schema (draft-07) is generated at compile time using [`schemars`].
//! Use [`generate_schema`] to obtain it at runtime, or find the pre-generated
//! copy in `schema/emp_mail.schema.json`.

pub mod error;
pub mod parser;
pub mod types;

pub use error::ParseError;
pub use parser::parse_eml;
pub use types::*;

/// Generate a JSON Schema (draft-07) for [`EmpMail`].
///
/// ```rust
/// let schema = enelyzer_ingestion_emp_mail::generate_schema();
/// println!("{}", serde_json::to_string_pretty(&schema).unwrap());
/// ```
pub fn generate_schema() -> serde_json::Value {
    let schema = schemars::schema_for!(EmpMail);
    serde_json::to_value(schema).expect("schema serialisation failed")
}
