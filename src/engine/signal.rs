//! إشارات الإجماع من `UnifiedMarketState` — تُستهلك حصراً في `decision.rs`.

use crate::config::USD_SCALE;
use crate::types::UnifiedMarketState;
use crate::validation::OrderSide;

#[inline]
pub fn price_from_scaled(scaled: i64) -> f64 {
    if scaled == 0 {
        return 0.0;
    }
    scaled as f64 / USD_SCALE
}

/// `consensus_score_scaled` في الحالة = طبقة تيكر (24س)؛ يُدمج مع CVD الحي والسعر.
pub fn resolve_consensus_from_market(market: &UnifiedMarketState) -> u32 {
    let mid = price_from_scaled(market.adjusted_price_scaled);
    let cvd = market.cvd_h1_scaled as f64 / USD_SCALE;
    let ticker_layer = market.consensus_score_scaled as f64;
    let cvd_norm = if mid > 0.0 { (cvd / mid).tanh() } else { 0.0 };
    let cvd_score = ((cvd_norm + 1.0) / 2.0) * 100.0 * 1_000_000.0;
    (ticker_layer * 0.55 + cvd_score * 0.45).round() as u32
}

/// هجوم شراء: CVD موجب؛ بيع: CVD سالب (دلتا تراكمية متوافقة).
pub fn cvd_aligned_with_side(side: OrderSide, cvd_h1_scaled: i64) -> bool {
    match side {
        OrderSide::Buy => cvd_h1_scaled > 0,
        OrderSide::Sell => cvd_h1_scaled < 0,
    }
}

/// السعر المعدّل صالح للتنفيذ.
pub fn price_lane_ready(adjusted_price_scaled: i64) -> bool {
    adjusted_price_scaled > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cvd_alignment() {
        assert!(cvd_aligned_with_side(OrderSide::Buy, 1_000_000));
        assert!(cvd_aligned_with_side(OrderSide::Sell, -1_000_000));
    }
}
