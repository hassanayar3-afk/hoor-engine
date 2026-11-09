//! تشغيل حُر: `demo` محلي، `once` / `run` مع WebSocket + استعادة إقلاع.

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use hoor_engine::exchange::{Credentials, FeedWarmupError, MarketFeed};
use hoor_engine::exchange::ws::run_market_stream;
use hoor_engine::pipeline::{CycleInput, MainLoopPipeline};
use hoor_engine::runner::{run_until_shutdown, HoorRunner, RunnerConfig};
use hoor_engine::types::UnifiedMarketState;
use hoor_engine::validation::OrderSide;
use hoor_engine::EngineConfig;

fn main() {
    let _ = dotenvy::dotenv();
    let mode = env::args().nth(1).unwrap_or_else(|| "demo".to_string());

    match mode.as_str() {
        "demo" => run_demo(),
        "once" => {
            if let Err(e) = run_binance_once() {
                eprintln!("once failed: {e}");
                std::process::exit(1);
            }
        }
        "run" => {
            if let Err(e) = run_binance_loop() {
                eprintln!("run failed: {e}");
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("usage: hoor-engine [demo|once|run]");
            eprintln!("unknown mode: {other}");
            std::process::exit(1);
        }
    }
}

fn run_demo() {
    let config = EngineConfig::default();
    let pipeline = MainLoopPipeline::new(config);

    let demo_input = CycleInput {
        market: UnifiedMarketState {
            symbol_hash: 0,
            timestamp_mono_ns: 0,
            adjusted_price_scaled: 50_000 * 100_000_000,
            cvd_h1_scaled: 1_000_000,
            consensus_score_scaled: 90_000_000,
            lock_status: 0,
        },
        btc_drop_shock: false,
        order_side: OrderSide::Buy,
        order_size: 1.0,
        bid_depth: 100.0,
        ask_depth: 100.0,
    };

    let outcome = pipeline.run_single_cycle(demo_input);
    eprintln!("HOOR v1.0 demo outcome: {:?}", outcome);

    let fraction = pipeline
        .engine
        .calculate_position_size(0.65, 2.0, 0.012);
    eprintln!(
        "Kelly fraction: {:.4} | balance USD: {:.2}",
        fraction,
        pipeline.engine.balance_usd()
    );
}

fn build_runner() -> Result<HoorRunner, Box<dyn std::error::Error>> {
    let runner_config = runner_config_from_env();
    let credentials = credentials_from_env();
    let pipeline = MainLoopPipeline::new(EngineConfig::default());
    let mut runner = HoorRunner::new(pipeline, runner_config.clone(), credentials)?;
    runner.startup_recovery()?;

    let feed = Arc::new(MarketFeed::new(runner_config.ws_freeze_timeout_ms));
    runner.attach_feed(Arc::clone(&feed));

  let endpoints = runner_config.endpoints.clone();
    let symbol = runner_config.trade_symbol.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("ws runtime");
        rt.block_on(async {
            let _ = run_market_stream(endpoints, &symbol, feed).await;
        });
    });

    Ok(runner)
}

fn ws_warmup_timeout() -> Duration {
    env::var("HOOR_WS_WARMUP_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_secs(5))
}

fn warmup_market_feed(runner: &HoorRunner) -> Result<(), FeedWarmupError> {
    let feed = runner
        .market_feed()
        .expect("FEED_WARMUP: internal error — feed must be attached in build_runner");
    let timeout = ws_warmup_timeout();
    feed.wait_for_live_ready(timeout)?;
    eprintln!("FEED_WARMUP: live bookTicker ready (timeout {}ms)", timeout.as_millis());
    Ok(())
}

fn run_binance_once() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = build_runner()?;
    warmup_market_feed(&runner)?;
    let outcome = runner.run_one_cycle()?;
    eprintln!("binance once outcome: {:?}", outcome);
    Ok(())
}

fn run_binance_loop() -> Result<(), Box<dyn std::error::Error>> {
    let runner = build_runner()?;
    warmup_market_feed(&runner)?;
    let shutdown = Arc::new(AtomicBool::new(false));
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let stop = Arc::clone(&shutdown);
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                stop.store(true, Ordering::Release);
            }
        });
        run_until_shutdown(runner, shutdown).await
    })?;
    Ok(())
}

fn runner_config_from_env() -> RunnerConfig {
    let mut cfg = RunnerConfig::default();
    if let Ok(s) = env::var("HOOR_SYMBOL") {
        cfg.trade_symbol = s;
    }
    if let Ok(b) = env::var("HOOR_BTC_SYMBOL") {
        cfg.btc_symbol = b;
    }
    if env::var("HOOR_TESTNET")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        cfg = cfg.testnet();
    }
    if let Ok(ms) = env::var("HOOR_WS_FREEZE_MS") {
        if let Ok(v) = ms.parse() {
            cfg.ws_freeze_timeout_ms = v;
        }
    }
    cfg
}

fn credentials_from_env() -> Credentials {
    Credentials::from_env().unwrap_or_else(|_| Credentials::market_data_only())
}
