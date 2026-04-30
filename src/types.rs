use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The root parsed structure of an EMP cockpit email.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmpMail {
    /// Standard email header fields.
    pub metadata: EmailMetadata,
    /// The report content extracted from the HTML body.
    pub report: Report,
}

/// Standard email header fields extracted from the raw email.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmailMetadata {
    /// Sender address (e.g. "Energy Cockpit <customer.page@energymarketprice.com>")
    pub from: String,
    /// Recipient address
    pub to: String,
    /// Email subject (decoded from encoded-word if necessary)
    pub subject: String,
    /// Date header value (e.g. "Wed, 15 Apr 2026 10:01:11 +0200")
    pub date: String,
    /// Message-ID header value
    pub message_id: Option<String>,
}

/// The structured report content extracted from the HTML body.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Report {
    /// Publication date displayed in the report header (e.g. "April 15, 2026")
    pub date: String,
    /// Market group title shown in the report header (e.g. "NLMK market update")
    pub title: String,
    /// News/article items included in the report.
    pub news_items: Vec<NewsItem>,
    /// Market price statistics table.
    pub statistics: Statistics,
}

/// A news article summarised in the report.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NewsItem {
    /// Date of the news item (e.g. "2026-04-15")
    pub date: String,
    /// Headline / title of the news article
    pub title: String,
    /// Short excerpt / summary (may be truncated in the email)
    pub summary: String,
    /// URL for the full article ("Read More" link)
    pub read_more_url: Option<String>,
}

/// The full statistics table from the report.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Statistics {
    /// The trading dates used as column headers (e.g. ["7/4", "8/4", ...])
    pub price_dates: Vec<String>,
    /// Note about the AVG/MAX/MIN columns
    pub ytd_note: String,
    /// Market sections (ELECTRICITY, GAS, OIL, OTHER)
    pub sections: Vec<MarketSection>,
}

/// A named group of market rows (e.g. "ELECTRICITY").
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MarketSection {
    /// Section name in uppercase (e.g. "ELECTRICITY", "GAS", "OIL", "OTHER")
    pub name: String,
    /// Rows of market price data within this section
    pub rows: Vec<MarketRow>,
}

/// A single market instrument row in the statistics table.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MarketRow {
    /// Instrument name (e.g. "BE spot", "IT M+1 base", "Gas BE ZTP M+2")
    pub market: String,
    /// Price unit (e.g. "EUR/MWh", "EUR/bbl", "EUR/tonne")
    pub unit: String,
    /// Daily closing prices for each date in `Statistics.price_dates`.
    /// `null` means no value was available for that date.
    pub prices: Vec<Option<f64>>,
    /// Day-over-day percentage change (e.g. -16.57 for -16.57%)
    pub var_d1_pct: f64,
    /// Week-over-week percentage change (e.g. 20.41 for 20.41%)
    pub var_w1_pct: f64,
    /// Year-to-date average price
    pub avg_ytd: f64,
    /// Year-to-date maximum price
    pub max_ytd: f64,
    /// Year-to-date minimum price
    pub min_ytd: f64,
    /// Trend direction derived from VAR D-1: "up", "down", or "flat"
    pub trend_d1: Trend,
    /// Trend direction derived from VAR W-1: "up", "down", or "flat"
    pub trend_w1: Trend,
}

/// Price movement direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Trend {
    Up,
    Down,
    Flat,
}

impl Trend {
    pub fn from_pct(pct: f64) -> Self {
        if pct > 0.0 {
            Trend::Up
        } else if pct < 0.0 {
            Trend::Down
        } else {
            Trend::Flat
        }
    }
}
