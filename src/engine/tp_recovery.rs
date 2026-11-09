use crate::validation::OrderSide;

/// استنتاج `tp_stage` من سعر الدخول والعلامة الحالية.
pub fn infer_tp_stage(
    side: OrderSide,
    entry: f64,
    mark: f64,
    tp1_pct: f64,
    tp2_pct: f64,
) -> u8 {
    if entry <= 0.0 || mark <= 0.0 {
        return 0;
    }
    let pnl_pct = match side {
        OrderSide::Buy => (mark - entry) / entry * 100.0,
        OrderSide::Sell => (entry - mark) / entry * 100.0,
    };
    if pnl_pct >= tp2_pct {
        2
    } else if pnl_pct >= tp1_pct {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_stage() {
        assert_eq!(infer_tp_stage(OrderSide::Buy, 100.0, 100.6, 0.5, 1.0), 1);
        assert_eq!(infer_tp_stage(OrderSide::Buy, 100.0, 102.0, 0.5, 1.0), 2);
    }
}
