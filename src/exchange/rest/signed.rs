use reqwest::blocking::Client;

use crate::exchange::credentials::Credentials;
use crate::exchange::error::ExchangeError;
use crate::exchange::rest::BinanceRestClient;
use crate::exchange::signing::sign_query;
use crate::guards::{ChronoGuard, LocalWeightBucket};

/// GET موقّع — يستهلك وزناً من الدلو أولاً.
pub fn signed_get_json(
    client: &BinanceRestClient,
    creds: &Credentials,
    bucket: &LocalWeightBucket,
    chrono: &ChronoGuard,
    path: &str,
    query_without_sig: &str,
    weight: u64,
) -> Result<serde_json::Value, ExchangeError> {
    if !creds.has_api_keys() {
        return Err(ExchangeError::MissingCredentials("HOOR_API_KEY"));
    }
    if !bucket.can_send_request(weight) {
        return Err(ExchangeError::WeightDenied { weight });
    }

    let ts = chrono
        .get_safe_exchange_timestamp()
        .map_err(ExchangeError::Other)?;
    let query = if query_without_sig.is_empty() {
        format!("timestamp={ts}&recvWindow=5000")
    } else {
        format!("{query_without_sig}&timestamp={ts}&recvWindow=5000")
    };
    let signature = sign_query(&creds.api_secret, &query);
    let url = format!(
        "{}{path}?{query}&signature={signature}",
        client.endpoints().rest_base
    );

    let http = Client::new();
    let resp = http
        .get(url)
        .header("X-MBX-APIKEY", &creds.api_key)
        .send()?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(ExchangeError::Api {
            code: status.as_u16() as i64,
            msg: text,
        });
    }
    Ok(resp.json()?)
}
