use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to parse email: {0}")]
    MailParse(#[from] mailparse::MailParseError),

    #[error("Email body is missing")]
    MissingBody,

    #[error("No HTML part found in email")]
    NoHtmlPart,

    #[error("Failed to decode quoted-printable body")]
    QuotedPrintableDecode,

    #[error("Failed to parse number '{value}': {source}")]
    NumberParse {
        value: String,
        source: std::num::ParseFloatError,
    },

    #[error("Statistics table not found in email body")]
    StatisticsTableNotFound,

    #[error("Expected {expected} columns but found {found} in row")]
    UnexpectedColumnCount { expected: usize, found: usize },
}
