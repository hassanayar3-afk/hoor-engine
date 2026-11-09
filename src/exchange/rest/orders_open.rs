use serde::Deserialize;

use crate::exchange::credentials::Credentials;
use crate::exchange::error::ExchangeError;
use crate::exchange::rest::signed::signed_get_json;
use crate::exchange::rest::BinanceRestClient;
use crate::guards::{ChronoGuard, LocalWeightBucket};

#[derive(Debug, Deserialize)]
struct OpenOrder {
    #[serde(rename = "orderId")]
    order_id: i64,
    symbol: String,
}

pub fn has_open_orders(
    client: &BinanceRestClient,
    creds: &Credentials,
    bucket: &LocalWeightBucket,
    chrono: &ChronoGuard,
    symbol: &str,
) -> Result<bool, ExchangeError> {
    use crate::exchange::binance_config::request_weight;

    let query = format!("symbol={symbol}");
    let v = signed_get_json(
        client,
        creds,
        bucket,
        chrono,
        "/fapi/v1/openOrders",
        &query,
        request_weight::OPEN_ORDERS,
    )?;
    let orders: Vec<OpenOrder> = serde_json::from_value(v)?;
    Ok(!orders.is_empty())
}
