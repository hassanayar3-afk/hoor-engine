//! طبقة باينانس — شبكة ومدخلات فقط؛ لا Kelly ولا `execute_main_cycle`.

pub mod binance_config;
mod credentials;
mod error;
mod exchange_info;
mod feed;
mod gateway;
mod market;
pub mod rest;
mod signing;
pub mod telegram;
pub mod ws;

pub use binance_config::{BinanceEndpoints, DEFAULT_DEPTH_LIMIT};
pub use credentials::Credentials;
pub use error::ExchangeError;
pub use exchange_info::{parse_symbol_filters, SymbolFilters};
pub use feed::{FeedWarmupError, MarketFeed};
pub use gateway::{BinanceGateway, DepthView};
pub use market::{btc_crash_shock_from_change_pct, consensus_from_ticker_strength};
pub use rest::BinanceRestClient;
