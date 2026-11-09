/// عناوين USDⓈ-M Futures — لا منطق شبكة.
#[derive(Debug, Clone)]
pub struct BinanceEndpoints {
    pub rest_base: String,
}

impl BinanceEndpoints {
    pub fn futures_mainnet() -> Self {
        Self {
            rest_base: "https://fapi.binance.com".to_string(),
        }
    }

    pub fn futures_testnet() -> Self {
        Self {
            rest_base: "https://testnet.binancefuture.com".to_string(),
        }
    }

    pub fn with_rest_base(rest_base: impl Into<String>) -> Self {
        Self {
            rest_base: rest_base.into(),
        }
    }
}

/// وزن الطلبات حسب وثائق باينانس (Futures).
pub mod request_weight {
    pub const SERVER_TIME: u64 = 1;
    pub const DEPTH_LIMIT_50: u64 = 2;
    pub const DEPTH_LIMIT_100: u64 = 5;
    pub const TICKER_24H: u64 = 1;
    pub const ORDER: u64 = 1;
    pub const KLINES_1H: u64 = 1;
    pub const ACCOUNT_BALANCE: u64 = 5;
    pub const POSITION_RISK: u64 = 5;
    pub const OPEN_ORDERS: u64 = 1;
    pub const EXCHANGE_INFO: u64 = 1;
}

pub const DEFAULT_DEPTH_LIMIT: u16 = 100;
