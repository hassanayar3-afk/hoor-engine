use crate::exchange::binance_config::request_weight;
use crate::exchange::binance_config::{BinanceEndpoints, DEFAULT_DEPTH_LIMIT};
use crate::exchange::error::ExchangeError;
use crate::exchange::credentials::Credentials;
use crate::exchange::rest::{
    atr_from_klines, depth_notional_sums, fetch_position, fetch_usdt_balance,
    mid_price, BinanceRestClient, DepthSnapshot, PositionSnapshot,
};
use crate::guards::{ChronoGuard, LocalWeightBucket};

#[derive(Debug, Clone)]
pub struct DepthView {
    pub bid_notional: f64,
    pub ask_notional: f64,
    pub mid_price: f64,
}

#[derive(Debug, Clone)]
pub struct BinanceGateway {
    client: BinanceRestClient,
}

impl BinanceGateway {
    pub fn new(endpoints: BinanceEndpoints) -> Result<Self, ExchangeError> {
        Ok(Self {
            client: BinanceRestClient::new(endpoints)?,
        })
    }

    pub fn client(&self) -> &BinanceRestClient {
        &self.client
    }

    pub fn sync_server_time(
        &self,
        bucket: &LocalWeightBucket,
        guard: &ChronoGuard,
    ) -> Result<(), ExchangeError> {
        if !bucket.can_send_request(request_weight::SERVER_TIME) {
            return Err(ExchangeError::WeightDenied {
                weight: request_weight::SERVER_TIME,
            });
        }
        let server_ms = self.client.get_server_time_ms()?;
        guard.update_offset(server_ms);
        Ok(())
    }

    fn fetch_depth(&self, bucket: &LocalWeightBucket, symbol: &str) -> Result<DepthSnapshot, ExchangeError> {
        let weight = request_weight::DEPTH_LIMIT_100;
        if !bucket.can_send_request(weight) {
            return Err(ExchangeError::WeightDenied { weight });
        }
        self.client.get_depth(symbol, DEFAULT_DEPTH_LIMIT)
    }

    pub fn fetch_depth_view(
        &self,
        bucket: &LocalWeightBucket,
        symbol: &str,
    ) -> Result<DepthView, ExchangeError> {
        let depth = self.fetch_depth(bucket, symbol)?;
        let (bid, ask) = depth_notional_sums(&depth);
        let mid = mid_price(&depth).unwrap_or(0.0);
        Ok(DepthView {
            bid_notional: bid,
            ask_notional: ask,
            mid_price: mid,
        })
    }

    pub fn fetch_atr_1h(&self, bucket: &LocalWeightBucket, symbol: &str) -> Result<f64, ExchangeError> {
        if !bucket.can_send_request(request_weight::KLINES_1H) {
            return Err(ExchangeError::WeightDenied {
                weight: request_weight::KLINES_1H,
            });
        }
        let klines = self.client.get_klines_1h(symbol, 32)?;
        Ok(atr_from_klines(&klines, 14))
    }

    pub fn fetch_atr_1h_only(
        &self,
        bucket: &LocalWeightBucket,
        symbol: &str,
    ) -> Result<f64, ExchangeError> {
        if !bucket.can_send_request(request_weight::KLINES_1H) {
            return Err(ExchangeError::WeightDenied {
                weight: request_weight::KLINES_1H,
            });
        }
        let klines = self.client.get_klines_1h(symbol, 32)?;
        Ok(atr_from_klines(&klines, 14))
    }

    pub fn sync_balance_usd(
        &self,
        creds: &Credentials,
        bucket: &LocalWeightBucket,
        chrono: &ChronoGuard,
    ) -> Result<f64, ExchangeError> {
        fetch_usdt_balance(self.client(), creds, bucket, chrono)
    }

    pub fn fetch_open_position(
        &self,
        creds: &Credentials,
        bucket: &LocalWeightBucket,
        chrono: &ChronoGuard,
        symbol: &str,
    ) -> Result<Option<PositionSnapshot>, ExchangeError> {
        fetch_position(self.client(), creds, bucket, chrono, symbol)
    }

    pub fn fetch_24h_change_pct(
        &self,
        bucket: &LocalWeightBucket,
        symbol: &str,
    ) -> Result<f64, ExchangeError> {
        if !bucket.can_send_request(request_weight::TICKER_24H) {
            return Err(ExchangeError::WeightDenied {
                weight: request_weight::TICKER_24H,
            });
        }
        self.client.get_ticker_price_change_percent(symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI64;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn sync_updates_guard() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = rt.block_on(MockServer::start());
        rt.block_on(async {
            Mock::given(method("GET"))
                .and(path("/fapi/v1/time"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "serverTime": 1_700_000_005_000u64
                })))
                .mount(&server)
                .await;
        });

        let gateway = BinanceGateway::new(BinanceEndpoints::with_rest_base(server.uri())).unwrap();
        let bucket = LocalWeightBucket::new();
        let offset = Arc::new(AtomicI64::new(0));
        let guard = ChronoGuard::new(offset);

        gateway.sync_server_time(&bucket, &guard).unwrap();
        assert!(guard.get_safe_exchange_timestamp().unwrap() > 1_600_000_000_000);
    }
}
