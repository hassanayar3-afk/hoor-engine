use serde::Deserialize;

use crate::exchange::credentials::Credentials;
use crate::exchange::SymbolFilters;
use crate::exchange::error::ExchangeError;
use crate::exchange::signing::sign_query;
use crate::exchange::rest::client::BinanceRestClient;
use crate::guards::{ChronoGuard, LocalWeightBucket};
use crate::validation::OrderSide;

#[derive(Debug, Clone, Copy)]
pub struct MarketOrderRequest<'a> {
    pub symbol: &'a str,
    pub side: OrderSide,
    pub quantity: f64,
    pub reduce_only: bool,
}

#[derive(Debug, Deserialize)]
pub struct OrderAck {
    #[serde(rename = "orderId")]
    pub order_id: i64,
    pub status: String,
}

pub struct OrderExecutor<'a> {
    client: &'a BinanceRestClient,
    credentials: &'a Credentials,
    bucket: &'a LocalWeightBucket,
    chrono: &'a ChronoGuard,
    filters: &'a SymbolFilters,
}

impl<'a> OrderExecutor<'a> {
    pub fn new(
        client: &'a BinanceRestClient,
        credentials: &'a Credentials,
        bucket: &'a LocalWeightBucket,
        chrono: &'a ChronoGuard,
        filters: &'a SymbolFilters,
    ) -> Self {
        Self {
            client,
            credentials,
            bucket,
            chrono,
            filters,
        }
    }

    pub fn place_market(&self, req: MarketOrderRequest<'_>) -> Result<OrderAck, ExchangeError> {
        use crate::exchange::binance_config::request_weight;

        if self.credentials.dry_run {
            return Ok(OrderAck {
                order_id: 0,
                status: "DRY_RUN_SIMULATED".to_string(),
            });
        }

        if !self.bucket.can_send_request(request_weight::ORDER) {
            return Err(ExchangeError::WeightDenied {
                weight: request_weight::ORDER,
            });
        }

        let qty = self.filters.round_qty(req.quantity);
        if qty < self.filters.min_qty {
            return Err(ExchangeError::Other("QTY_BELOW_MIN"));
        }

        let ts = self
            .chrono
            .get_safe_exchange_timestamp()
            .map_err(ExchangeError::Other)?;
        let side = match req.side {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        };
        let qty_s = self.filters.format_qty(qty);
        let mut query = format!(
            "symbol={}&side={}&type=MARKET&quantity={}&timestamp={}&recvWindow=5000",
            req.symbol, side, qty_s, ts
        );
        if req.reduce_only {
            query.push_str("&reduceOnly=true");
        }
        let signature = sign_query(&self.credentials.api_secret, &query);
        let url = format!(
            "{}/fapi/v1/order?{query}&signature={signature}",
            self.client.endpoints().rest_base
        );

        let http = reqwest::blocking::Client::new();
        let resp = http
            .post(url)
            .header("X-MBX-APIKEY", &self.credentials.api_key)
            .send()?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(ExchangeError::Api {
                code: status.as_u16() as i64,
                msg: text,
            });
        }

        let ack: OrderAck = resp.json()?;
        Ok(ack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::binance_config::BinanceEndpoints;
    use crate::exchange::SymbolFilters;
    use crate::exchange::rest::BinanceRestClient;
    use crate::guards::{ChronoGuard, LocalWeightBucket};
    use std::sync::atomic::AtomicI64;
    use std::sync::Arc;

    fn test_filters() -> SymbolFilters {
        SymbolFilters {
            step_size: 0.001,
            min_qty: 0.001,
            min_notional: 5.0,
            tick_size: 0.1,
            qty_precision: 3,
        }
    }

    #[test]
    fn dry_run_never_hits_network() {
        let client =
            BinanceRestClient::new(BinanceEndpoints::futures_mainnet()).expect("client");
        let creds = Credentials::market_data_only();
        let bucket = LocalWeightBucket::new();
        let guard = ChronoGuard::new(Arc::new(AtomicI64::new(0)));
        let filters = test_filters();
        let exec = OrderExecutor::new(&client, &creds, &bucket, &guard, &filters);
        let ack = exec
            .place_market(MarketOrderRequest {
                symbol: "BTCUSDT",
                side: OrderSide::Buy,
                quantity: 0.001,
                reduce_only: false,
            })
            .unwrap();
        assert_eq!(ack.status, "DRY_RUN_SIMULATED");
    }
}
