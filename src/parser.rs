use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

use crate::error::ParseError;
use crate::types::{
    EmailMetadata, EmpMail, MarketRow, MarketSection, NewsItem, Report, Statistics, Trend,
};

static RE_PCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([+-]?\d+(?:\.\d+)?)%$").unwrap());
static RE_UNIT: Lazy<Regex> = Lazy::new(|| Regex::new(r"\(([^)]+)\)\s*$").unwrap());

/// Parse a raw `.eml` file (bytes) into an [`EmpMail`] struct.
///
/// # Errors
/// Returns [`ParseError`] when the email cannot be parsed or required
/// sections are missing from the HTML body.
pub fn parse_eml(raw: &[u8]) -> Result<EmpMail, ParseError> {
    let parsed = mailparse::parse_mail(raw)?;
    let metadata = extract_metadata(&parsed)?;
    let html_body = extract_html_body(&parsed)?;
    let report = parse_html_body(&html_body)?;
    Ok(EmpMail { metadata, report })
}

fn extract_metadata(mail: &mailparse::ParsedMail<'_>) -> Result<EmailMetadata, ParseError> {
    let headers = &mail.headers;
    let get = |name: &str| -> String {
        headers
            .iter()
            .find(|h| h.get_key_ref().eq_ignore_ascii_case(name))
            .map(|h| h.get_value())
            .unwrap_or_default()
    };

    Ok(EmailMetadata {
        from: get("From"),
        to: get("To"),
        subject: get("Subject"),
        date: get("Date"),
        message_id: {
            let v = get("Message-ID");
            if v.is_empty() { None } else { Some(v) }
        },
    })
}

/// Walk the MIME tree to find the first `text/html` body part.
fn extract_html_body(mail: &mailparse::ParsedMail<'_>) -> Result<String, ParseError> {
    fn find_html<'a>(part: &'a mailparse::ParsedMail<'a>) -> Option<String> {
        if part
            .ctype
            .mimetype
            .eq_ignore_ascii_case("text/html")
        {
            return part.get_body().ok();
        }
        for sub in &part.subparts {
            if let Some(html) = find_html(sub) {
                return Some(html);
            }
        }
        None
    }

    find_html(mail).ok_or(ParseError::NoHtmlPart)
}

fn parse_html_body(html: &str) -> Result<Report, ParseError> {
    let doc = Html::parse_document(html);

    let date = extract_report_date(&doc);
    let title = extract_report_title(&doc);
    let news_items = extract_news_items(&doc);
    let statistics = extract_statistics(&doc)?;

    Ok(Report { date, title, news_items, statistics })
}

fn extract_report_date(doc: &Html) -> String {
    let sel = Selector::parse("p[style*='margin:40px']").unwrap();
    doc.select(&sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default()
}

fn extract_report_title(doc: &Html) -> String {
    let sel =
        Selector::parse("p[style*='font-weight:700'][style*='uppercase']").unwrap();
    doc.select(&sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_default()
}

fn extract_news_items(doc: &Html) -> Vec<NewsItem> {
    let mut items: Vec<NewsItem> = Vec::new();

    // News articles are laid out in cards; identify by the date cell style.
    let date_sel =
        Selector::parse("td[style*='color:#576B75'][style*='font-size: 12px']").unwrap();
    let link_sel = Selector::parse("a").unwrap();

    for date_td in doc.select(&date_sel) {
        let date = date_td.text().collect::<String>().trim().to_string();
        if date.is_empty() || !date.chars().any(|c| c.is_ascii_digit()) {
            continue;
        }

        // Navigate: parent tbody → sibling rows
        let parent_table = date_td.parent().and_then(|tr| tr.parent()); // <tbody> or <table>
        if parent_table.is_none() {
            continue;
        }

        // Collect all text siblings in the same card table: title row, summary row, link row
        let mut title = String::new();
        let mut summary = String::new();
        let mut read_more_url: Option<String> = None;

        // Walk siblings from same ancestor table to find title / summary / link
        let title_sel =
            Selector::parse("td[style*='height:80px']").unwrap();
        let summary_sel =
            Selector::parse("td[style*='height:144px']").unwrap();

        // Find the enclosing card table (width=90%)
        let mut ancestor = date_td.parent(); // <tr>
        for _ in 0..4 {
            ancestor = ancestor.and_then(|n| n.parent());
        }
        if let Some(card) = ancestor {
            let card_html = card.html();
            let card_doc = Html::parse_fragment(&card_html);

            if let Some(t) = card_doc.select(&title_sel).next() {
                title = t.text().collect::<String>().trim().to_string();
            }
            if let Some(s) = card_doc.select(&summary_sel).next() {
                summary = s.text().collect::<String>().trim().to_string();
            }
            if let Some(a) = card_doc.select(&link_sel).next() {
                read_more_url = a
                    .value()
                    .attr("originalsrc")
                    .or_else(|| a.value().attr("href"))
                    .map(|s| s.to_string());
            }
        }

        if !title.is_empty() {
            items.push(NewsItem { date, title, summary, read_more_url });
        }
    }

    items
}

fn extract_statistics(doc: &Html) -> Result<Statistics, ParseError> {
    // Locate the STATISTICS section by its heading text.
    let p_sel = Selector::parse("p").unwrap();
    let stats_heading = doc.select(&p_sel).find(|e| {
        e.text().collect::<String>().trim().eq_ignore_ascii_case("STATISTICS")
    });

    // Find the note about AVG/MAX/MIN
    let note_sel = Selector::parse("p[style*='text-align:right']").unwrap();
    let ytd_note = doc
        .select(&note_sel)
        .next()
        .map(|e| e.text().collect::<String>().trim().to_string())
        .unwrap_or_else(|| "* MAX, MIN, AVG - Since the beginning of the year".to_string());

    if stats_heading.is_none() {
        return Err(ParseError::StatisticsTableNotFound);
    }

    // Extract column header dates (7/4, 8/4, ...) from header row cells.
    let price_dates = extract_price_dates(doc);

    // Parse market sections and rows.
    let sections = extract_market_sections(doc)?;

    Ok(Statistics { price_dates, ytd_note, sections })
}

fn extract_price_dates(doc: &Html) -> Vec<String> {
    let header_sel = Selector::parse(
        "td[bgcolor='#330a19'] font",
    )
    .unwrap();

    doc.select(&header_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .filter(|s| s.contains('/'))
        .collect()
}

fn extract_market_sections(doc: &Html) -> Result<Vec<MarketSection>, ParseError> {
    let mut sections: Vec<MarketSection> = Vec::new();

    // Section header rows: bgcolor=#330a19 with colspan=12 containing the section name.
    let section_header_sel = Selector::parse("td[bgcolor='#330a19'][colspan='12']").unwrap();
    let data_row_sel_light = Selector::parse("tr").unwrap();

    // We'll iterate through all table rows in document order.
    let tr_sel = Selector::parse("tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();

    let mut current_section: Option<String> = None;
    let mut rows_buffer: Vec<MarketRow> = Vec::new();

    for tr in doc.select(&tr_sel) {
        let tr_html = tr.html();
        let tr_frag = Html::parse_fragment(&tr_html);

        // Check if this row is a section header (has a td with colspan=12 and bgcolor=#330a19)
        let is_section_header = tr_frag
            .select(&section_header_sel)
            .next()
            .is_some();

        if is_section_header {
            // Save previous section
            if let Some(name) = current_section.take() {
                if !rows_buffer.is_empty() {
                    sections.push(MarketSection {
                        name,
                        rows: std::mem::take(&mut rows_buffer),
                    });
                }
            }

            // Extract section name from the nested text
            let section_name = tr_frag
                .select(&Selector::parse("td").unwrap())
                .flat_map(|td| td.text())
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_uppercase();

            // Clean: "ELECTRICITY", "GAS", "OIL", "OTHER"
            let clean_name = extract_section_name(&section_name);
            current_section = Some(clean_name);
            continue;
        }

        // Check if this is a data row (has many td cells with price data).
        // Data rows have alternating bgcolor: #F0F3F4 and white, with 11 columns.
        let tds: Vec<_> = tr_frag.select(&td_sel).collect();
        if tds.len() < 11 || current_section.is_none() {
            continue;
        }

        // First td: market name + unit
        let first_td_html = tds[0].html();
        let (market, unit) = parse_market_name(&first_td_html);

        if market.is_empty() {
            continue;
        }

        // Columns 2–7 (indices 1–6): daily prices
        let mut prices: Vec<Option<f64>> = Vec::new();
        for i in 1..=6 {
            let cell_text = tds[i].text().collect::<String>().trim().to_string();
            prices.push(parse_optional_f64(&cell_text));
        }

        // Column 8 (index 7): VAR D-1
        let var_d1_text = tds[7].text().collect::<String>().trim().to_string();
        let var_d1_pct = parse_pct(&var_d1_text).unwrap_or(0.0);

        // Column 9 (index 8): VAR W-1
        let var_w1_text = tds[8].text().collect::<String>().trim().to_string();
        let var_w1_pct = parse_pct(&var_w1_text).unwrap_or(0.0);

        // Column 10 (index 9): AVG
        let avg_ytd = tds[9]
            .text()
            .collect::<String>()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);

        // Column 11 (index 10): MAX
        let max_ytd = tds[10]
            .text()
            .collect::<String>()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);

        // Column 12 (index 11): MIN
        let min_ytd = if tds.len() > 11 {
            tds[11]
                .text()
                .collect::<String>()
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        rows_buffer.push(MarketRow {
            market,
            unit,
            prices,
            var_d1_pct,
            var_w1_pct,
            avg_ytd,
            max_ytd,
            min_ytd,
            trend_d1: Trend::from_pct(var_d1_pct),
            trend_w1: Trend::from_pct(var_w1_pct),
        });
    }

    // Flush last section
    if let Some(name) = current_section {
        if !rows_buffer.is_empty() {
            sections.push(MarketSection { name, rows: rows_buffer });
        }
    }

    Ok(sections)
}

fn extract_section_name(raw: &str) -> String {
    // Strip noise and extract just the section keyword
    for keyword in &["ELECTRICITY", "GAS", "OIL", "OTHER"] {
        if raw.contains(keyword) {
            return keyword.to_string();
        }
    }
    raw.to_string()
}

fn parse_market_name(td_html: &str) -> (String, String) {
    let frag = Html::parse_fragment(td_html);
    let root = frag.root_element();
    let full_text = root.text().collect::<Vec<_>>();

    // Unit is inside <font> tag; market name is the non-unit text
    let font_sel = Selector::parse("font").unwrap();
    let unit = frag
        .select(&font_sel)
        .next()
        .map(|e| {
            e.text()
                .collect::<String>()
                .trim()
                .trim_matches('(')
                .trim_matches(')')
                .to_string()
        })
        .unwrap_or_default();

    // Market name: all text minus the unit portion, trimmed
    let raw_full = full_text
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let unit_with_parens = format!("({})", unit);
    let market = raw_full
        .replace(&unit_with_parens, "")
        .trim()
        .to_string();

    (market, unit)
}

fn parse_optional_f64(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    if trimmed.is_empty() || trimmed == "&nbsp;" || trimmed == "\u{a0}" {
        None
    } else {
        trimmed.parse::<f64>().ok()
    }
}

fn parse_pct(s: &str) -> Option<f64> {
    let trimmed = s.trim();
    RE_PCT.captures(trimmed).and_then(|c| c[1].parse::<f64>().ok())
}
