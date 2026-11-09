use serde::Deserialize;

use crate::exchange::error::ExchangeError;

#[derive(Debug, Clone)]
pub struct SymbolFilters {
    pub step_size: f64,
    pub min_qty: f64,
    pub min_notional: f64,
    pub tick_size: f64,
    pub qty_precision: usize,
}

impl SymbolFilters {
    pub fn round_qty(&self, qty: f64) -> f64 {
        if self.step_size <= 0.0 {
            return qty;
        }
        let steps = (qty / self.step_size).floor();
        let rounded = steps * self.step_size;
        rounded.max(self.min_qty)
    }

    pub fn format_qty(&self, qty: f64) -> String {
        format!("{:.*}", self.qty_precision, self.round_qty(qty))
    }

    pub fn validate_notional(&self, qty: f64, price: f64) -> Result<f64, ExchangeError> {
        let q = self.round_qty(qty);
        if q * price < self.min_notional {
            return Err(ExchangeError::Other("MIN_NOTIONAL_REJECT"));
        }
        Ok(q)
    }
}

#[derive(Debug, Deserialize)]
struct ExchangeInfoResponse {
    symbols: Vec<SymbolEntry>,
}

#[derive(Debug, Deserialize)]
struct SymbolEntry {
    symbol: String,
    filters: Vec<serde_json::Value>,
}

pub fn parse_symbol_filters(json: &serde_json::Value, symbol: &str) -> Result<SymbolFilters, ExchangeError> {
    let root: ExchangeInfoResponse = serde_json::from_value(json.clone())?;
    let entry = root
        .symbols
        .into_iter()
        .find(|s| s.symbol == symbol)
        .ok_or(ExchangeError::Other("SYMBOL_NOT_IN_EXCHANGE_INFO"))?;

    let mut step_size = 0.001;
    let mut min_qty = 0.001;
    let mut min_notional = 5.0;
    let mut tick_size = 0.01;

    for f in entry.filters {
        let ftype = f.get("filterType").and_then(|x| x.as_str()).unwrap_or("");
        match ftype {
            "LOT_SIZE" => {
                step_size = parse_filter_f64(&f, "stepSize", step_size);
                min_qty = parse_filter_f64(&f, "minQty", min_qty);
            }
            "MIN_NOTIONAL" => {
                min_notional = parse_filter_f64(&f, "notional", min_notional);
            }
            "PRICE_FILTER" => {
                tick_size = parse_filter_f64(&f, "tickSize", tick_size);
            }
            _ => {}
        }
    }

    let qty_precision = precision_from_step(step_size);

    Ok(SymbolFilters {
        step_size,
        min_qty,
        min_notional,
        tick_size,
        qty_precision,
    })
}

fn parse_filter_f64(v: &serde_json::Value, key: &str, default: f64) -> f64 {
    v.get(key)
        .and_then(|x| x.as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn precision_from_step(step: f64) -> usize {
    if step >= 1.0 {
        return 0;
    }
    let s = format!("{step}");
    if let Some(i) = s.find('.') {
        s[i + 1..].trim_end_matches('0').len()
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_to_step() {
        let f = SymbolFilters {
            step_size: 0.001,
            min_qty: 0.001,
            min_notional: 5.0,
            tick_size: 0.1,
            qty_precision: 3,
        };
        assert!((f.round_qty(0.1234) - 0.123).abs() < 1e-9);
    }
}
