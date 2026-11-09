use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExchangeError {
    #[error("HTTP transport: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON decode: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RATE_LIMIT_GUARD: weight bucket rejected request ({weight})")]
    WeightDenied { weight: u64 },

    #[error("Binance API status {code}: {msg}")]
    Api { code: i64, msg: String },

    #[error("missing credentials: {0}")]
    MissingCredentials(&'static str),

    #[error("{0}")]
    Other(&'static str),
}
