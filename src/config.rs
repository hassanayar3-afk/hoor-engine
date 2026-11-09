//! ثوابت الدستور السيادي — لا منطق تشغيل هنا.

/// رأس المال الافتراضي (USD) حسب المواصفة.
pub const DEFAULT_INITIAL_BALANCE_USD: f64 = 500.0;

/// سقف وزن الطلبات المحلي (من 6000 باينانس).
pub const WEIGHT_CEILING: u64 = 4000;

/// وزن افتراضي لطلب واحد في دورة القرار.
pub const DEFAULT_REQUEST_WEIGHT: u64 = 5;

/// نافذة إعادة ضبط عداد الوزن (ms).
pub const WEIGHT_RESET_WINDOW_MS: u64 = 60_000;

/// أدنى إجماع للهجوم التلقائي (×10⁶ → 85%).
pub const CONSENSUS_ATTACK_THRESHOLD: u32 = 85_000_000;

/// نطاق التنبيه اليدوي فقط (80–84%).
pub const CONSENSUS_ALERT_THRESHOLD: u32 = 80_000_000;

/// Scale للأسعار والأرصدة ($ × 10⁸).
pub const USD_SCALE: f64 = 100_000_000.0;

/// إعدادات تشغيل قابلة للتمرير من `main` فقط.
#[derive(Debug, Clone, Copy)]
pub struct EngineConfig {
    pub initial_balance_usd: f64,
    pub request_weight_per_cycle: u64,
    pub consensus_threshold: u32,
    pub consensus_alert_threshold: u32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            initial_balance_usd: DEFAULT_INITIAL_BALANCE_USD,
            request_weight_per_cycle: DEFAULT_REQUEST_WEIGHT,
            consensus_threshold: CONSENSUS_ATTACK_THRESHOLD,
            consensus_alert_threshold: CONSENSUS_ALERT_THRESHOLD,
        }
    }
}
