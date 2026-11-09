/// حجم المركز: Kelly جزئي (30%) مع سقف ATR ومخاطر 1–3%.

pub fn fractional_kelly_position_fraction(win_rate: f64, win_loss_ratio: f64, atr: f64) -> f64 {
    let p = win_rate.clamp(0.55, 0.85);
    let b = win_loss_ratio.clamp(1.2, 3.0);
    let q = 1.0 - p;
    let raw_kelly = (b * p - q) / b;
    let fractional_kelly = raw_kelly * 0.30;
    let volatility_scalar = (0.015 / atr.max(0.003)).clamp(0.4, 1.2);
    let calculated = fractional_kelly * volatility_scalar;
    calculated.clamp(0.01, 0.03)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_risk_band() {
        let f = fractional_kelly_position_fraction(0.7, 2.0, 0.01);
        assert!((0.01..=0.03).contains(&f));
    }
}
