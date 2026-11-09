//! سجل الصفقات المغلقة — Kelly ديناميكي من الأداء الفعلي.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const JOURNAL_PATH: &str = "hoor_trade_journal.json";
const MIN_TRADES_FOR_DYNAMIC: u32 = 5;
const DEFAULT_WIN_RATE: f64 = 0.62;
const DEFAULT_WIN_LOSS_RATIO: f64 = 2.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TradeJournal {
    pub wins: u32,
    pub losses: u32,
    pub sum_win_pnl_pct: f64,
    pub sum_loss_pnl_pct: f64,
}

impl TradeJournal {
    pub fn load() -> Self {
        let path = PathBuf::from(JOURNAL_PATH);
        if !path.exists() {
            return Self::default();
        }
        fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(JOURNAL_PATH, json)
    }

    pub fn record_close(&mut self, pnl_pct: f64) {
        if pnl_pct >= 0.0 {
            self.wins += 1;
            self.sum_win_pnl_pct += pnl_pct;
        } else {
            self.losses += 1;
            self.sum_loss_pnl_pct += pnl_pct.abs();
        }
        let _ = self.save();
    }

    pub fn dynamic_win_rate(&self) -> f64 {
        let n = self.wins + self.losses;
        if n < MIN_TRADES_FOR_DYNAMIC {
            return DEFAULT_WIN_RATE;
        }
        self.wins as f64 / n as f64
    }

    pub fn dynamic_win_loss_ratio(&self) -> f64 {
        let n = self.wins + self.losses;
        if n < MIN_TRADES_FOR_DYNAMIC || self.wins == 0 || self.losses == 0 {
            return DEFAULT_WIN_LOSS_RATIO;
        }
        let avg_win = self.sum_win_pnl_pct / self.wins as f64;
        let avg_loss = self.sum_loss_pnl_pct / self.losses as f64;
        if avg_loss <= f64::EPSILON {
            return DEFAULT_WIN_LOSS_RATIO;
        }
        (avg_win / avg_loss).clamp(1.2, 3.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_before_enough_trades() {
        let j = TradeJournal::default();
        assert_eq!(j.dynamic_win_rate(), DEFAULT_WIN_RATE);
    }
}
