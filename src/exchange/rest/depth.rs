use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct DepthSnapshot {
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepthLevel(
    #[serde(deserialize_with = "deserialize_decimal_string")] pub f64,
    #[serde(deserialize_with = "deserialize_decimal_string")] pub f64,
);

fn deserialize_decimal_string<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f64>().map_err(serde::de::Error::custom)
}

/// أفضل سعر وسيط من أول مستوى في الدفتر.
pub fn mid_price(depth: &DepthSnapshot) -> Option<f64> {
    let bid = depth.bids.first()?.0;
    let ask = depth.asks.first()?.0;
    if bid > 0.0 && ask > 0.0 {
        Some((bid + ask) / 2.0)
    } else {
        None
    }
}

/// مجموع العمق بالدولار التقريبي (price × qty) لكل جانب.
pub fn depth_notional_sums(depth: &DepthSnapshot) -> (f64, f64) {
    let bid = depth
        .bids
        .iter()
        .map(|lvl| lvl.0 * lvl.1)
        .sum::<f64>();
    let ask = depth
        .asks
        .iter()
        .map(|lvl| lvl.0 * lvl.1)
        .sum::<f64>();
    (bid, ask)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_notional() {
        let depth = DepthSnapshot {
            bids: vec![DepthLevel(100.0, 2.0), DepthLevel(50.0, 1.0)],
            asks: vec![DepthLevel(101.0, 1.0)],
        };
        let (b, a) = depth_notional_sums(&depth);
        assert!((b - 250.0).abs() < f64::EPSILON);
        assert!((a - 101.0).abs() < f64::EPSILON);
    }
}
