use std::env;

use crate::exchange::error::ExchangeError;

#[derive(Debug, Clone)]
pub struct Credentials {
    pub api_key: String,
    pub api_secret: String,
    pub dry_run: bool,
}

impl Credentials {
    pub fn from_env() -> Result<Self, ExchangeError> {
        let api_key = env::var("HOOR_API_KEY")
            .or_else(|_| env::var("BINANCE_API_KEY"))
            .map_err(|_| ExchangeError::MissingCredentials("HOOR_API_KEY/BINANCE_API_KEY"))?;
        let api_secret = env::var("HOOR_API_SECRET")
            .or_else(|_| env::var("BINANCE_API_SECRET"))
            .map_err(|_| ExchangeError::MissingCredentials("HOOR_API_SECRET/BINANCE_API_SECRET"))?;
        let dry_run = env::var("HOOR_DRY_RUN")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(true);
        Ok(Self {
            api_key,
            api_secret,
            dry_run,
        })
    }

    /// تشغيل بدون مفاتيح — قراءة سوق فقط.
    pub fn market_data_only() -> Self {
        Self {
            api_key: String::new(),
            api_secret: String::new(),
            dry_run: true,
        }
    }

    pub fn has_api_keys(&self) -> bool {
        !self.api_key.is_empty() && !self.api_secret.is_empty()
    }
}
