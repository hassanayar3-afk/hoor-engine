#[derive(Debug, Clone)]
pub struct Kline {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// ATR بسيط (متوسط المدى الحقيقي لآخر `period` شموع).
pub fn atr_from_klines(klines: &[Kline], period: usize) -> f64 {
    if klines.len() < period + 1 {
        return 0.01;
    }
    let start = klines.len().saturating_sub(period);
    let mut tr_sum = 0.0;
    for i in start..klines.len() {
        let prev_close = klines[i - 1].close;
        let k = &klines[i];
        let tr = (k.high - k.low)
            .max((k.high - prev_close).abs())
            .max((k.low - prev_close).abs());
        tr_sum += tr;
    }
    let atr = tr_sum / period as f64;
    atr.max(0.003)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atr_positive() {
        let klines = vec![
            Kline {
                open: 100.0,
                high: 102.0,
                low: 99.0,
                close: 101.0,
                volume: 10.0,
            },
            Kline {
                open: 101.0,
                high: 103.0,
                low: 100.0,
                close: 102.0,
                volume: 11.0,
            },
        ];
        let atr = atr_from_klines(&klines, 1);
        assert!(atr > 0.0);
    }
}
