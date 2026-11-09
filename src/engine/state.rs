use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

use crate::config::USD_SCALE;
use crate::engine::kelly::fractional_kelly_position_fraction;
use crate::guards::LocalWeightBucket;

/// حالة المحرك الذرية — أعلام الصفقة والطوارئ والرصيد.
pub struct HoorTradingEngine {
    pub is_position_active: AtomicBool,
    pub emergency_halt: AtomicBool,
    pub weight_bucket: LocalWeightBucket,
    pub account_balance_scaled: AtomicI64,
}

impl HoorTradingEngine {
    pub fn new(initial_balance_usd: f64) -> Self {
        Self {
            is_position_active: AtomicBool::new(false),
            emergency_halt: AtomicBool::new(false),
            weight_bucket: LocalWeightBucket::new(),
            account_balance_scaled: AtomicI64::new((initial_balance_usd * USD_SCALE) as i64),
        }
    }

    /// نسبة المحفظة المخصصة للصفقة (ليس حجم العقد بالدولار مباشرة).
    pub fn calculate_position_size(&self, win_rate: f64, win_loss_ratio: f64, atr: f64) -> f64 {
        fractional_kelly_position_fraction(win_rate, win_loss_ratio, atr)
    }

    pub fn balance_usd(&self) -> f64 {
        self.account_balance_scaled.load(Ordering::Acquire) as f64 / USD_SCALE
    }

    pub fn set_emergency_halt(&self, halt: bool) {
        self.emergency_halt.store(halt, Ordering::SeqCst);
    }

    pub fn release_position_lock(&self) {
        self.is_position_active.store(false, Ordering::SeqCst);
    }

    pub fn set_balance_usd(&self, usd: f64) {
        self.account_balance_scaled
            .store((usd * USD_SCALE) as i64, Ordering::SeqCst);
    }
}
