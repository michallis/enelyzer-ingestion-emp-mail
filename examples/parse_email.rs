//! Example: parse an EMP cockpit `.eml` file and print a summary + JSON.
//!
//! ```bash
//! cargo run --example parse_email -- path/to/My_Energy_Cockpit.eml
//! cargo run --example parse_email -- path/to/My_Energy_Cockpit.eml --json
//! cargo run --example parse_email -- path/to/My_Energy_Cockpit.eml --schema
//! ```

use enelyzer_ingestion_emp_mail::{generate_schema, parse_eml};
use std::{env, fs};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: parse_email <path/to/file.eml> [--json | --schema]");
        std::process::exit(1);
    }

    // Schema-only mode
    if args.iter().any(|a| a == "--schema") {
        let schema = generate_schema();
        println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        return;
    }

    let path = &args[1];
    let json_mode = args.iter().any(|a| a == "--json");

    let raw = fs::read(path).unwrap_or_else(|e| {
        eprintln!("Cannot read '{}': {}", path, e);
        std::process::exit(1);
    });

    let mail = parse_eml(&raw).unwrap_or_else(|e| {
        eprintln!("Parse error: {}", e);
        std::process::exit(1);
    });

    if json_mode {
        println!("{}", serde_json::to_string_pretty(&mail).unwrap());
        return;
    }

    // Human-readable summary
    println!("=== EMAIL METADATA ===");
    println!("  From       : {}", mail.metadata.from);
    println!("  To         : {}", mail.metadata.to);
    println!("  Subject    : {}", mail.metadata.subject);
    println!("  Date       : {}", mail.metadata.date);
    if let Some(id) = &mail.metadata.message_id {
        println!("  Message-ID : {}", id);
    }

    println!("\n=== REPORT ===");
    println!("  Date  : {}", mail.report.date);
    println!("  Title : {}", mail.report.title);

    println!("\n=== NEWS ITEMS ({}) ===", mail.report.news_items.len());
    for (i, item) in mail.report.news_items.iter().enumerate() {
        println!("  [{}] {} — {}", i + 1, item.date, item.title);
        if !item.summary.is_empty() {
            let snippet: String = item.summary.chars().take(100).collect();
            println!("      {}{}", snippet, if item.summary.len() > 100 { "…" } else { "" });
        }
    }

    println!("\n=== STATISTICS ===");
    println!("  Note   : {}", mail.report.statistics.ytd_note);
    let dates = mail.report.statistics.price_dates.join(" | ");
    println!("  Dates  : {}", dates);

    for section in &mail.report.statistics.sections {
        println!("\n  ── {} ──", section.name);
        println!(
            "  {:<35} {:<12} {:>10} {:>10} {:>10} {:>10} {:>10}",
            "Market", "Unit", "Last Price", "VAR D-1%", "VAR W-1%", "AVG YTD", "MAX YTD"
        );
        println!("  {}", "-".repeat(100));
        for row in &section.rows {
            let last_price = row
                .prices
                .iter()
                .rev()
                .find_map(|p| *p)
                .map(|v| format!("{:.2}", v))
                .unwrap_or_else(|| "N/A".to_string());

            println!(
                "  {:<35} {:<12} {:>10} {:>+9.2}% {:>+9.2}% {:>10.2} {:>10.2}",
                row.market,
                row.unit,
                last_price,
                row.var_d1_pct,
                row.var_w1_pct,
                row.avg_ytd,
                row.max_ytd,
            );
        }
    }

    println!("\nTotal market rows: {}", mail
        .report
        .statistics
        .sections
        .iter()
        .map(|s| s.rows.len())
        .sum::<usize>()
    );
}
