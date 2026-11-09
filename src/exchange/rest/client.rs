use crate::exchange::binance_config::BinanceEndpoints;
use crate::exchange::error::ExchangeError;
use crate::exchange::rest::{DepthSnapshot, ServerTimeResponse};

#[derive(Debug, Clone)]
pub struct BinanceRestClient {
    http: reqwest::blocking::Client,
    endpoints: BinanceEndpoints,
}

impl BinanceRestClient {
    pub fn new(endpoints: BinanceEndpoints) -> Result<Self, ExchangeError> {
        let http = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        Ok(Self { http, endpoints })
    }

    pub fn endpoints(&self) -> &BinanceEndpoints {
        &self.endpoints
    }

    pub fn get_server_time_ms(&self) -> Result<u64, ExchangeError> {
        let url = format!("{}/fapi/v1/time", self.endpoints.rest_base);
        let resp = self.http.get(url).send()?;
        if !resp.status().is_success() {
            return Err(ExchangeError::Other("SERVER_TIME_HTTP_FAILURE"));
        }
        let body: ServerTimeResponse = resp.json()?;
        Ok(body.server_time_ms)
    }

    pub fn get_depth(&self, symbol: &str, limit: u16) -> Result<DepthSnapshot, ExchangeError> {
        let url = format!(
            "{}/fapi/v1/depth?symbol={symbol}&limit={limit}",
            self.endpoints.rest_base
        );
        let resp = self.http.get(url).send()?;
        if !resp.status().is_success() {
            return Err(ExchangeError::Other("DEPTH_HTTP_FAILURE"));
        }
        let body: DepthSnapshot = resp.json()?;
        Ok(body)
    }

    pub fn get_ticker_price_change_percent(&self, symbol: &str) -> Result<f64, ExchangeError> {
        let url = format!(
            "{}/fapi/v1/ticker/24hr?symbol={symbol}",
            self.endpoints.rest_base
        );
        let resp = self.http.get(url).send()?;
        if !resp.status().is_success() {
            return Err(ExchangeError::Other("TICKER_HTTP_FAILURE"));
        }
        let v: serde_json::Value = resp.json()?;
        let pct = v
            .get("priceChangePercent")
            .and_then(|x| x.as_str())
            .ok_or(ExchangeError::Other("TICKER_FIELD_MISSING"))?
            .parse::<f64>()
            .map_err(|_| ExchangeError::Other("TICKER_PARSE_FAILURE"))?;
        Ok(pct)
    }

    pub fn get_klines_1h(&self, symbol: &str, limit: u16) -> Result<Vec<super::klines::Kline>, ExchangeError> {
        let url = format!(
            "{}/fapi/v1/klines?symbol={symbol}&interval=1h&limit={limit}",
            self.endpoints.rest_base
        );
        let resp = self.http.get(url).send()?;
        if !resp.status().is_success() {
            return Err(ExchangeError::Other("KLINES_HTTP_FAILURE"));
        }
        let raw: Vec<Vec<serde_json::Value>> = resp.json()?;
        let mut out = Vec::with_capacity(raw.len());
        for row in raw {
            if row.len() < 6 {
                continue;
            }
            let parse = |v: &serde_json::Value| -> Option<f64> {
                v.as_str()?.parse().ok()
            };
            out.push(super::klines::Kline {
                open: parse(&row[1]).unwrap_or(0.0),
                high: parse(&row[2]).unwrap_or(0.0),
                low: parse(&row[3]).unwrap_or(0.0),
                close: parse(&row[4]).unwrap_or(0.0),
                volume: parse(&row[5]).unwrap_or(0.0),
            });
        }
        Ok(out)
    }

    pub fn get_exchange_info_json(&self) -> Result<serde_json::Value, ExchangeError> {
        let url = format!("{}/fapi/v1/exchangeInfo", self.endpoints.rest_base);
        let resp = self.http.get(url).send()?;
        if !resp.status().is_success() {
            return Err(ExchangeError::Other("EXCHANGE_INFO_HTTP_FAILURE"));
        }
        Ok(resp.json()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exchange::binance_config::DEFAULT_DEPTH_LIMIT;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn parses_server_time() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = rt.block_on(MockServer::start());
        rt.block_on(async {
            Mock::given(method("GET"))
                .and(path("/fapi/v1/time"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "serverTime": 1_700_000_000_000u64
                })))
                .mount(&server)
                .await;
        });

        let client =
            BinanceRestClient::new(BinanceEndpoints::with_rest_base(server.uri())).unwrap();
        assert_eq!(client.get_server_time_ms().unwrap(), 1_700_000_000_000);
    }

    #[test]
    fn parses_depth() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = rt.block_on(MockServer::start());
        rt.block_on(async {
            Mock::given(method("GET"))
                .and(path("/fapi/v1/depth"))
                .and(query_param("symbol", "BTCUSDT"))
                .and(query_param("limit", DEFAULT_DEPTH_LIMIT.to_string().as_str()))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "bids": [["50000.0", "1.0"]],
                    "asks": [["50001.0", "2.0"]]
                })))
                .mount(&server)
                .await;
        });

        let client =
            BinanceRestClient::new(BinanceEndpoints::with_rest_base(server.uri())).unwrap();
        let depth = client.get_depth("BTCUSDT", DEFAULT_DEPTH_LIMIT).unwrap();
        assert_eq!(depth.bids.len(), 1);
        assert_eq!(depth.asks.len(), 1);
    }
}
