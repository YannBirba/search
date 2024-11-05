use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Parsing failed: {0}")]
    ParsingError(String),

    #[error("Rate limited")]
    RateLimited,
}