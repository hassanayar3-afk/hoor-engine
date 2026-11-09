use serde::Deserialize;

use crate::exchange::credentials::Credentials;
use crate::exchange::error::ExchangeError;
use crate::exchange::rest::signed::signed_get_json;
use crate::exchange::rest::BinanceRestClient;
use crate::guards::{ChronoGuard, LocalWeightBucket};

#[derive(Debug, Clone)]
pub struct PositionSnapshot {
    pub position_amt: f64,
    pub entry_price: f64,
    pub mark_price: f64,
}

#[derive(Debug, Deserialize)]
struct BalanceRow {
    asset: String,
    #[serde(rename = "balance")]
    balance: String,
}

#[derive(Debug, Deserialize)]
struct PositionRow {
  symbol: String,
  #[serde(rename = "positionAmt")]
    position_amt: String,
    #[serde(rename = "entryPrice")]
    entry_price: String,
    #[serde(rename = "markPrice")]
    mark_price: String,
}

pub fn fetch_usdt_balance(
    client: &BinanceRestClient,
    creds: &Credentials,
    bucket: &LocalWeightBucket,
    chrono: &ChronoGuard,
) -> Result<f64, ExchangeError> {
    use crate::exchange::binance_config::request_weight;

    let v = signed_get_json(
        client,
        creds,
        bucket,
        chrono,
        "/fapi/v2/balance",
        "",
        request_weight::ACCOUNT_BALANCE,
    )?;
    let rows: Vec<BalanceRow> = serde_json::from_value(v)?;
    for row in rows {
        if row.asset == "USDT" {
            return row
                .balance
                .parse::<f64>()
                .map_err(|_| ExchangeError::Other("BALANCE_PARSE_FAILURE"));
        }
    }
    Err(ExchangeError::Other("USDT_BALANCE_NOT_FOUND"))
}

pub fn fetch_position(
    client: &BinanceRestClient,
    creds: &Credentials,
    bucket: &LocalWeightBucket,
    chrono: &ChronoGuard,
    symbol: &str,
) -> Result<Option<PositionSnapshot>, ExchangeError> {
    use crate::exchange::binance_config::request_weight;

    let query = format!("symbol={symbol}");
    let v = signed_get_json(
        client,
        creds,
        bucket,
        chrono,
        "/fapi/v2/positionRisk",
        &query,
        request_weight::POSITION_RISK,
    )?;
    let rows: Vec<PositionRow> = serde_json::from_value(v)?;
    for row in rows {
        if row.symbol != symbol {
            continue;
        }
        let amt = row
            .position_amt
            .parse::<f64>()
            .map_err(|_| ExchangeError::Other("POSITION_AMT_PARSE"))?;
        if amt.abs() < 1e-12 {
            return Ok(None);
        }
        return Ok(Some(PositionSnapshot {
            position_amt: amt,
            entry_price: row.entry_price.parse().unwrap_or(0.0),
            mark_price: row.mark_price.parse().unwrap_or(0.0),
        }));
    }
    Ok(None)
}
