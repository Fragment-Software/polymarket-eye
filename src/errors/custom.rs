use eyre::Report;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(unused)]
pub enum CustomError {
    #[error("Deserialization error: {0}")]
    Deserialize(#[from] serde_json::Error),

    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Amount of tries is exceeded")]
    TriesExceeded,

    #[error("Polymarket API error: {0}")]
    PolymarketApi(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Clob API error: {0}")]
    ClobApiError(String),

    #[error("Unexpected error: {0}")]
    Unexpected(#[from] Report),
}
