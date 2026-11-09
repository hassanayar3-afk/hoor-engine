use crate::exchange::SymbolFilters;

/// حجم العقد من نسبة Kelly مع فلاتر البورصة.
pub fn contract_quantity(
    balance_usd: f64,
    mid_price: f64,
    kelly_fraction: f64,
    filters: &SymbolFilters,
) -> f64 {
    if mid_price <= 0.0 || balance_usd <= 0.0 {
        return filters.min_qty;
    }
    let notional = balance_usd * kelly_fraction;
    let raw = notional / mid_price;
    match filters.validate_notional(raw, mid_price) {
        Ok(q) => q,
        Err(_) => filters.min_qty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::SymbolFilters;

  #[test]
    fn respects_filters() {
        let f = SymbolFilters {
            step_size: 0.001,
            min_qty: 0.001,
            min_notional: 5.0,
            tick_size: 0.1,
            qty_precision: 3,
        };
        let q = contract_quantity(500.0, 50_000.0, 0.02, &f);
        assert!(q >= f.min_qty);
    }
}
