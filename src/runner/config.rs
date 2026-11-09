use std::time::Duration;

use crate::exchange::SymbolFilters;
use crate::exchange::BinanceEndpoints;
use crate::validation::OrderSide;

#[derive(Debug, Clone)]
pub struct RunnerConfig {
    pub endpoints: BinanceEndpoints,
    pub trade_symbol: String,
    pub btc_symbol: String,
    pub default_side: OrderSide,
    pub reference_order_size: f64,
    pub cycle_interval: Duration,
    pub btc_shock_threshold_pct: f64,
    pub use_testnet: bool,
    pub win_rate: f64,
    pub win_loss_ratio: f64,
    pub stop_loss_pct: f64,
    pub tp1_pct: f64,
    pub tp2_pct: f64,
    pub tp3_trail_pct: f64,
    pub tp_leg_fraction: f64,
    pub qty_step: f64,
    pub ws_freeze_timeout_ms: u64,
}

pub const EXCHANGE_FATAL_THRESHOLD: u32 = 5;
/// تنبيه تيليجرام عند تجاوز هذا العدد من أخطاء البورصة المتتالية.
pub const EXCHANGE_CRITICAL_ALERT_THRESHOLD: u32 = 3;

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            endpoints: BinanceEndpoints::futures_mainnet(),
            trade_symbol: "BTCUSDT".to_string(),
            btc_symbol: "BTCUSDT".to_string(),
            default_side: OrderSide::Buy,
            reference_order_size: 0.001,
            cycle_interval: Duration::from_secs(5),
            btc_shock_threshold_pct: -3.0,
            use_testnet: false,
            win_rate: 0.62,
            win_loss_ratio: 2.0,
            stop_loss_pct: 1.0,
            tp1_pct: 0.5,
            tp2_pct: 1.0,
            tp3_trail_pct: 0.3,
            tp_leg_fraction: 0.33,
            qty_step: 0.001,
            ws_freeze_timeout_ms: 8_000,
        }
    }
}

impl RunnerConfig {
    pub fn testnet(mut self) -> Self {
        self.use_testnet = true;
        self.endpoints = BinanceEndpoints::futures_testnet();
        self
    }

    pub fn fallback_filters(&self) -> SymbolFilters {
        SymbolFilters {
            step_size: self.qty_step,
            min_qty: self.qty_step,
            min_notional: 5.0,
            tick_size: 0.01,
            qty_precision: 3,
        }
    }
}
