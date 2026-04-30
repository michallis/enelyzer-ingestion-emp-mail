use once_cell::sync::Lazy;
use regex::Regex;
use scraper::{Html, Selector};

use crate::error::ParseError;
use crate::types::{
    EmailMetadata, EmpMail, MarketRow, MarketSection, NewsItem, Report, Statistics, Trend,
};

static RE_PCT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([+-]?\d+(?:\.\d+)?)%$").unwrap());

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
    // Each news card contains four characteristic cells in order:
    //   1. Date   td[style*="color:#576B75"]
    //   2. Title  td[style*="height:80px"]
    //   3. Summary td[style*="height:144px"]
    //   4. Link   td > a  (Read More)
    //
    // We collect all four lists and zip them by position.

    let date_sel =
        Selector::parse("td[style*='color:#576B75'][style*='font-size: 12px']").unwrap();
    let title_sel = Selector::parse("td[style*='height:80px']").unwrap();
    let summary_sel = Selector::parse("td[style*='height:144px']").unwrap();
    let link_td_sel = Selector::parse("td[style*='text-transform:uppercase'] a").unwrap();

    let dates: Vec<String> = doc
        .select(&date_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .filter(|s| s.chars().any(|c| c.is_ascii_digit()))
        .collect();

    let titles: Vec<String> = doc
        .select(&title_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let summaries: Vec<String> = doc
        .select(&summary_sel)
        .map(|e| e.text().collect::<String>().trim().to_string())
        .collect();

    let links: Vec<Option<String>> = doc
        .select(&link_td_sel)
        .map(|a| {
            a.value()
                .attr("originalsrc")
                .or_else(|| a.value().attr("href"))
                .map(|s| s.to_string())
        })
        .collect();

    let n = dates.len().min(titles.len());
    (0..n)
        .map(|i| NewsItem {
            date: dates[i].clone(),
            title: titles[i].clone(),
            summary: summaries.get(i).cloned().unwrap_or_default(),
            read_more_url: links.get(i).cloned().flatten(),
        })
        .collect()
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
    use scraper::ElementRef;

    let mut sections: Vec<MarketSection> = Vec::new();

    // Selectors used within each row's subtree (no fragment re-parsing needed).
    let section_header_sel =
        Selector::parse("td[bgcolor='#330a19'][colspan='12']").unwrap();
    let tr_sel = Selector::parse("tr").unwrap();

    let mut current_section: Option<String> = None;
    let mut rows_buffer: Vec<MarketRow> = Vec::new();

    for tr in doc.select(&tr_sel) {
        // Check for section header: a <td bgcolor="#330a19" colspan="12"> directly inside this <tr>.
        let is_section_header = tr.select(&section_header_sel).next().is_some();

        if is_section_header {
            // Flush previous section.
            if let Some(name) = current_section.take() {
                if !rows_buffer.is_empty() {
                    sections.push(MarketSection {
                        name,
                        rows: std::mem::take(&mut rows_buffer),
                    });
                }
            }

            // Extract section name from all text inside this row.
            let section_name = tr
                .text()
                .map(|t| t.trim())
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
                .to_uppercase();

            current_section = Some(extract_section_name(&section_name));
            continue;
        }

        if current_section.is_none() {
            continue;
        }

        // Get only direct <td> children of this <tr> (avoids picking up nested cells).
        let tds: Vec<ElementRef> = tr
            .children()
            .filter_map(ElementRef::wrap)
            .filter(|e| e.value().name() == "td")
            .collect();

        // Data rows must have exactly 11 or 12 columns.
        if tds.len() < 11 {
            continue;
        }

        // First td: market name + unit.
        let (market, unit) = parse_market_name_from_elem(tds[0]);
        if market.is_empty() {
            continue;
        }

        // Columns 2–7 (indices 1–6): daily prices.
        let prices: Vec<Option<f64>> = (1..=6)
            .map(|i| parse_optional_f64(&tds[i].text().collect::<String>()))
            .collect();

        // Column 8 (index 7): VAR D-1.
        let var_d1_pct =
            parse_pct(&tds[7].text().collect::<String>()).unwrap_or(0.0);

        // Column 9 (index 8): VAR W-1.
        let var_w1_pct =
            parse_pct(&tds[8].text().collect::<String>()).unwrap_or(0.0);

        // Column 10 (index 9): AVG.
        let avg_ytd = tds[9]
            .text()
            .collect::<String>()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);

        // Column 11 (index 10): MAX.
        let max_ytd = tds[10]
            .text()
            .collect::<String>()
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);

        // Column 12 (index 11): MIN.
        let min_ytd = tds
            .get(11)
            .map(|td| {
                td.text()
                    .collect::<String>()
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0);

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

    // Flush last section.
    if let Some(name) = current_section {
        if !rows_buffer.is_empty() {
            sections.push(MarketSection { name, rows: rows_buffer });
        }
    }

    Ok(sections)
}

/// Extract market name and unit from a `<td>` element directly.
fn parse_market_name_from_elem(td: scraper::ElementRef) -> (String, String) {
    let font_sel = Selector::parse("font").unwrap();

    // Unit is inside <font> tag.
    let unit = td
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

    // All text of the td, minus the unit portion.
    let full_text: String = td
        .text()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    let unit_parens = format!("({})", unit);
    let market = full_text.replace(&unit_parens, "").trim().to_string();

    (market, unit)
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
