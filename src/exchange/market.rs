//! تحويل بيانات السوق إلى مدخلات الحلقة — ليس قرار تداول.

/// يحوّل تغيّر السعر 24س (-5%..+5%) إلى `consensus_score_scaled` (0..100 × 10⁶).
pub fn consensus_from_ticker_strength(change_pct_24h: f64) -> u32 {
    let normalized = ((change_pct_24h + 5.0) / 10.0).clamp(0.0, 1.0);
    (normalized * 100.0 * 1_000_000.0).round() as u32
}

/// صدمة ماكرو: BTC تحت عتبة (مثلاً -3% خلال 24س).
pub fn btc_crash_shock_from_change_pct(btc_change_pct_24h: f64, threshold_pct: f64) -> bool {
    btc_change_pct_24h <= threshold_pct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_uptrend_high_consensus() {
        let score = consensus_from_ticker_strength(5.0);
        assert_eq!(score, 100_000_000);
    }

    #[test]
    fn btc_shock() {
        assert!(btc_crash_shock_from_change_pct(-4.0, -3.0));
        assert!(!btc_crash_shock_from_change_pct(-2.0, -3.0));
    }
}
