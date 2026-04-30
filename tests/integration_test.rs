use enelyzer_ingestion_emp_mail::{generate_schema, parse_eml, EmpMail};

/// Minimal valid EML that exercises all parsing paths.
const MINIMAL_EML: &str = r##"From: "Energy Cockpit" <customer.page@energymarketprice.com>
To: user@example.com
Subject: My Energy Cockpit
Date: Wed, 15 Apr 2026 10:01:11 +0200
Message-ID: <test-001@example>
Content-Type: text/html; charset="utf-8"
Content-Transfer-Encoding: quoted-printable
MIME-Version: 1.0

<!DOCTYPE HTML><html><head></head><body>
<p style="margin:40px 0 0 0px">April 15, 2026</p>
<p style="margin:0 0 20px 0; font-weight:700; text-transform: uppercase; font-size: 25px; line-height: 30px;">NLMK market update</p>
<p><i>* MAX, MIN, AVG - Since the beginning of the year</i></p>
<p style="font-size:12px; text-align:right; color:gray; margin:0;"><i>* MAX, MIN, AVG - Since the beginning of the year</i></p>
<p style="margin:20px 0 15px">STATISTICS</p>
<table>
  <tr>
    <td bgcolor="#330a19">Markets</td>
    <td bgcolor="#330a19"><font style="font-weight:400">7/4</font><br>Tue</td>
    <td bgcolor="#330a19"><font style="font-weight:400">8/4</font><br>Wed</td>
    <td bgcolor="#330a19"><font style="font-weight:400">9/4</font><br>Thu</td>
    <td bgcolor="#330a19"><font style="font-weight:400">10/4</font><br>Fri</td>
    <td bgcolor="#330a19"><font style="font-weight:400">13/4</font><br>Mon</td>
    <td bgcolor="#330a19"><font style="font-weight:400">14/4</font><br>Tue</td>
    <td bgcolor="#330a19">VAR<br>D-1</td>
    <td bgcolor="#330a19">VAR<br>W-1</td>
    <td bgcolor="#330a19">AVG</td>
    <td bgcolor="#330a19">MAX</td>
    <td bgcolor="#330a19" style="border-radius:0 5px 0 0;">MIN</td>
  </tr>
  <tr>
    <td bgcolor="#330a19" colspan="12">
      <table><tr><td>&nbsp;&nbsp;Electricity</td></tr></table>
    </td>
  </tr>
  <tr>
    <td bgcolor="#F0F3F4" style="font-weight:bold; height: 32px;">&nbsp;&nbsp;BE spot <font style="font-weight:400">(Eur/MWh)</font></td>
    <td bgcolor="#F0F3F4">86.12</td>
    <td bgcolor="#F0F3F4">78.70</td>
    <td bgcolor="#F0F3F4">113.93</td>
    <td bgcolor="#F0F3F4">48.76</td>
    <td bgcolor="#F0F3F4">124.29</td>
    <td bgcolor="#F0F3F4">103.70</td>
    <td bgcolor="#00BEA7">-16.57%</td>
    <td bgcolor="#FF1F55">20.41%</td>
    <td bgcolor="#F0F3F4">94.56</td>
    <td bgcolor="#F0F3F4">146.22</td>
    <td bgcolor="#F0F3F4">0.05</td>
  </tr>
  <tr>
    <td bgcolor="#330a19" colspan="12">
      <table><tr><td>&nbsp;&nbsp;Gas</td></tr></table>
    </td>
  </tr>
  <tr>
    <td style="font-weight:bold; height: 32px;">&nbsp;&nbsp;Gas BE ZTP spot <font style="font-weight:400">(EUR/MWh)</font></td>
    <td>52.21</td>
    <td>45.47</td>
    <td>45.97</td>
    <td>44.32</td>
    <td>47.05</td>
    <td>42.89</td>
    <td bgcolor="#00BEA7">-8.84%</td>
    <td bgcolor="#00BEA7">-17.84%</td>
    <td>40.78</td>
    <td>61.24</td>
    <td>28.74</td>
  </tr>
</table>
</body></html>
"##;

fn parsed_minimal() -> EmpMail {
    parse_eml(MINIMAL_EML.as_bytes()).expect("minimal EML should parse without errors")
}

#[test]
fn test_metadata_from() {
    let mail = parsed_minimal();
    assert!(
        mail.metadata.from.contains("energymarketprice.com"),
        "From should contain the sender domain"
    );
}

#[test]
fn test_metadata_to() {
    let mail = parsed_minimal();
    assert_eq!(mail.metadata.to, "user@example.com");
}

#[test]
fn test_metadata_subject() {
    let mail = parsed_minimal();
    assert!(!mail.metadata.subject.is_empty(), "Subject should not be empty");
}

#[test]
fn test_metadata_date() {
    let mail = parsed_minimal();
    assert!(!mail.metadata.date.is_empty(), "Date should not be empty");
}

#[test]
fn test_metadata_message_id() {
    let mail = parsed_minimal();
    assert!(mail.metadata.message_id.is_some(), "Message-ID should be parsed");
}

#[test]
fn test_report_date_extracted() {
    let mail = parsed_minimal();
    assert!(
        mail.report.date.contains("2026"),
        "Report date should contain the year"
    );
}

#[test]
fn test_report_title_extracted() {
    let mail = parsed_minimal();
    let title = &mail.report.title;
    assert!(!title.is_empty(), "Report title should not be empty");
    assert!(
        title.to_uppercase().contains("MARKET"),
        "Title should contain 'MARKET', got: '{}'",
        title
    );
}

#[test]
fn test_statistics_sections_present() {
    let mail = parsed_minimal();
    assert!(
        !mail.report.statistics.sections.is_empty(),
        "Statistics should contain at least one section"
    );
}

#[test]
fn test_electricity_section() {
    let mail = parsed_minimal();
    let elec = mail
        .report
        .statistics
        .sections
        .iter()
        .find(|s| s.name == "ELECTRICITY");
    assert!(elec.is_some(), "ELECTRICITY section should be present");
    let rows = &elec.unwrap().rows;
    assert!(!rows.is_empty(), "ELECTRICITY section should have rows");
}

#[test]
fn test_gas_section() {
    let mail = parsed_minimal();
    let gas = mail
        .report
        .statistics
        .sections
        .iter()
        .find(|s| s.name == "GAS");
    assert!(gas.is_some(), "GAS section should be present");
}

#[test]
fn test_be_spot_values() {
    let mail = parsed_minimal();
    let elec = mail
        .report
        .statistics
        .sections
        .iter()
        .find(|s| s.name == "ELECTRICITY")
        .unwrap();
    let be_spot = elec.rows.iter().find(|r| r.market.contains("BE spot")).unwrap();

    assert!((be_spot.var_d1_pct - (-16.57)).abs() < 0.01, "var_d1_pct should be -16.57");
    assert!((be_spot.var_w1_pct - 20.41).abs() < 0.01, "var_w1_pct should be 20.41");
    assert!((be_spot.avg_ytd - 94.56).abs() < 0.01, "avg_ytd should be 94.56");
    assert!((be_spot.max_ytd - 146.22).abs() < 0.01, "max_ytd should be 146.22");
    assert!((be_spot.min_ytd - 0.05).abs() < 0.01, "min_ytd should be 0.05");
    assert_eq!(be_spot.unit, "Eur/MWh");
    assert_eq!(be_spot.prices.len(), 6);
}

#[test]
fn test_trend_directions() {
    use enelyzer_ingestion_emp_mail::Trend;

    let mail = parsed_minimal();
    let elec = mail
        .report
        .statistics
        .sections
        .iter()
        .find(|s| s.name == "ELECTRICITY")
        .unwrap();
    let be_spot = elec.rows.iter().find(|r| r.market.contains("BE spot")).unwrap();

    assert_eq!(be_spot.trend_d1, Trend::Down, "trend_d1 should be Down for -16.57%");
    assert_eq!(be_spot.trend_w1, Trend::Up, "trend_w1 should be Up for +20.41%");
}

#[test]
fn test_price_dates_extracted() {
    let mail = parsed_minimal();
    let dates = &mail.report.statistics.price_dates;
    assert!(!dates.is_empty(), "Price dates should be extracted");
    // Our minimal fixture has 7/4, 8/4, 9/4, 10/4, 13/4, 14/4
    assert!(dates.iter().any(|d| d.contains('/')), "Dates should contain '/'");
}

#[test]
fn test_ytd_note_present() {
    let mail = parsed_minimal();
    let note = &mail.report.statistics.ytd_note;
    assert!(!note.is_empty(), "YTD note should not be empty");
}

#[test]
fn test_roundtrip_json() {
    let mail = parsed_minimal();
    let json = serde_json::to_string(&mail).expect("serialisation should succeed");
    let back: EmpMail = serde_json::from_str(&json).expect("deserialisation should succeed");
    assert_eq!(mail.metadata.from, back.metadata.from);
    assert_eq!(mail.report.title, back.report.title);
    assert_eq!(
        mail.report.statistics.sections.len(),
        back.report.statistics.sections.len()
    );
}

#[test]
fn test_generate_schema_is_valid_json() {
    let schema = generate_schema();
    assert!(schema.is_object(), "Schema should be a JSON object");
}

/// Run this test against a real EML file by setting the env var:
///
/// ```bash
/// EMP_FIXTURE_PATH=/path/to/My_Energy_Cockpit.eml cargo test test_real_file -- --ignored
/// ```
#[test]
#[ignore = "requires EMP_FIXTURE_PATH env var to be set to a real .eml file path"]
fn test_real_file() {
    let path = std::env::var("EMP_FIXTURE_PATH")
        .expect("Set EMP_FIXTURE_PATH to a real .eml file path");
    let raw = std::fs::read(&path).expect("Cannot read file");
    let mail = parse_eml(&raw).expect("Real EML should parse without errors");

    println!("Date  : {}", mail.report.date);
    println!("Title : {}", mail.report.title);
    println!("News  : {}", mail.report.news_items.len());
    println!("Sections: {}", mail.report.statistics.sections.len());
    for s in &mail.report.statistics.sections {
        println!("  [{}] {} rows", s.name, s.rows.len());
    }

    let json = serde_json::to_string_pretty(&mail).unwrap();
    println!("{}", &json[..json.len().min(2000)]);

    assert!(!mail.report.statistics.sections.is_empty());
}
