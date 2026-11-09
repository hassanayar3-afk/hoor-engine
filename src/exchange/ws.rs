use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::exchange::binance_config::BinanceEndpoints;
use crate::exchange::error::ExchangeError;
use crate::exchange::feed::MarketFeed;

fn ws_base(endpoints: &BinanceEndpoints) -> &str {
    if endpoints.rest_base.contains("testnet") {
        "wss://stream.binancefuture.com"
    } else {
        "wss://fstream.binance.com"
    }
}

/// بث مركّب: bookTicker (سعر) + aggTrade (CVD حي).
pub async fn run_market_stream(
    endpoints: BinanceEndpoints,
    symbol: &str,
    feed: Arc<MarketFeed>,
) -> Result<(), ExchangeError> {
    let sym = symbol.to_lowercase();
    let url = format!(
        "{}/stream?streams={sym}@bookTicker/{sym}@aggTrade",
        ws_base(&endpoints)
    );
    loop {
        match connect_async(&url).await {
            Ok((mut ws, _)) => {
                while let Some(msg) = ws.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                let data = v.get("data").unwrap_or(&v);
                                if data.get("b").is_some() && data.get("a").is_some() {
                                    let bid = data
                                        .get("b")
                                        .and_then(|x| x.as_str())
                                        .and_then(|s| s.parse::<f64>().ok());
                                    let ask = data
                                        .get("a")
                                        .and_then(|x| x.as_str())
                                        .and_then(|s| s.parse::<f64>().ok());
                                    if let (Some(b), Some(a)) = (bid, ask) {
                                        feed.update_mid((b + a) / 2.0);
                                    }
                                } else if let (Some(q), Some(m)) = (
                                    data.get("q").and_then(|x| x.as_str()),
                                    data.get("m").and_then(|x| x.as_bool()),
                                ) {
                                    if let Ok(qty) = q.parse::<f64>() {
                                        feed.apply_agg_trade(qty, m);
                                    }
                                }
                            }
                        }
                        Ok(Message::Ping(p)) => {
                            let _ = ws.send(Message::Pong(p)).await;
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
            }
            Err(e) => {
                eprintln!("ws connect error: {e}");
            }
        }
        feed.mark_disconnected();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}
