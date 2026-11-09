use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use crate::config::USD_SCALE;
use crate::engine::{resolve_consensus_from_market, TradeJournal};
use crate::exchange::telegram::{
    notify_emergency_halt, notify_exchange_failure_streak, notify_stream_freeze,
    send_manual_alert,
};
use crate::exchange::{
    btc_crash_shock_from_change_pct, consensus_from_ticker_strength, BinanceGateway,
    Credentials, ExchangeError, MarketFeed,
};
use crate::exchange::rest::{MarketOrderRequest, OrderExecutor};
use crate::exchange::SymbolFilters;
use crate::pipeline::{CycleInput, CycleOutcome, MainLoopPipeline};
use crate::runner::exit::run_exit_cycle;
use crate::runner::position::TrackedPosition;
use crate::runner::sizing::contract_quantity;
use crate::types::UnifiedMarketState;
use super::RunnerConfig;

#[derive(Debug, thiserror::Error)]
pub enum CycleError {
    #[error(transparent)]
    Exchange(#[from] ExchangeError),
    #[error("pipeline halted: {0:?}")]
    Halted(CycleOutcome),
}

pub struct HoorRunner {
    pub pipeline: MainLoopPipeline,
    pub gateway: BinanceGateway,
    pub config: RunnerConfig,
    pub credentials: Credentials,
    pub filters: SymbolFilters,
    pub feed: Option<Arc<MarketFeed>>,
    pub journal: TradeJournal,
    last_time_sync: Option<Instant>,
    pub(crate) tracked: Option<TrackedPosition>,
    exchange_error_streak: u32,
}

impl HoorRunner {
    pub fn new(
        pipeline: MainLoopPipeline,
        config: RunnerConfig,
        credentials: Credentials,
    ) -> Result<Self, ExchangeError> {
        let gateway = BinanceGateway::new(config.endpoints.clone())?;
        let filters = config.fallback_filters();
        Ok(Self {
            pipeline,
            gateway,
            config,
            credentials,
            filters,
            feed: None,
            journal: TradeJournal::load(),
            last_time_sync: None,
            tracked: None,
            exchange_error_streak: 0,
        })
    }

    pub fn attach_feed(&mut self, feed: Arc<MarketFeed>) {
        self.feed = Some(feed);
    }

    pub fn market_feed(&self) -> Option<Arc<MarketFeed>> {
        self.feed.clone()
    }

    fn maybe_sync_time(&mut self) -> Result<(), ExchangeError> {
        let need = self
            .last_time_sync
            .map(|t| t.elapsed() >= self.pipeline.chrono.recalibration_interval())
            .unwrap_or(true);

        if need {
            self.gateway.sync_server_time(
                &self.pipeline.engine.weight_bucket,
                &self.pipeline.chrono,
            )?;
            self.last_time_sync = Some(Instant::now());
        }
        Ok(())
    }

    fn sync_balance_if_possible(&mut self) {
        if !self.credentials.has_api_keys() {
            return;
        }
        let bucket = &self.pipeline.engine.weight_bucket;
        match self.gateway.sync_balance_usd(
            &self.credentials,
            bucket,
            &self.pipeline.chrono,
        ) {
            Ok(usd) => self.pipeline.engine.set_balance_usd(usd),
            Err(e) => eprintln!("balance sync skipped: {e}"),
        }
    }

    fn on_exchange_error(&mut self, err: &ExchangeError) {
        self.exchange_error_streak += 1;
        if self.exchange_error_streak > super::config::EXCHANGE_CRITICAL_ALERT_THRESHOLD {
            let _ = notify_exchange_failure_streak(self.exchange_error_streak, &format!("{err}"));
        }
        if self.exchange_error_streak >= super::config::EXCHANGE_FATAL_THRESHOLD {
            self.pipeline.engine.set_emergency_halt(true);
            eprintln!("emergency_halt after repeated errors: {err}");
            let _ = notify_emergency_halt(&format!("Repeated exchange errors: {err}"));
        }
    }

    fn clear_error_streak(&mut self) {
        self.exchange_error_streak = 0;
    }

    fn map_exchange<T>(&mut self, result: Result<T, ExchangeError>) -> Result<T, CycleError> {
        match result {
            Ok(v) => Ok(v),
            Err(e) => {
                self.on_exchange_error(&e);
                Err(CycleError::Exchange(e))
            }
        }
    }

    fn live_feed(&self) -> Result<Arc<MarketFeed>, CycleError> {
        self.feed
            .as_ref()
            .cloned()
            .ok_or(CycleError::Halted(CycleOutcome::Halted(
                "NO_FEED: WebSocket market feed required.",
            )))
    }

    fn resolve_mid(&self, feed: &MarketFeed, rest_mid: f64) -> Result<f64, CycleError> {
        if feed.check_freeze_watchdog() {
            let _ = notify_stream_freeze(
                "WebSocket watchdog tripped or no ticks within freeze timeout.",
            );
            return Err(CycleError::Halted(CycleOutcome::Halted(
                "STREAM_FREEZE: WebSocket watchdog tripped.",
            )));
        }
        if let Some(mid) = feed.mid_price() {
            return Ok(mid);
        }
        if rest_mid > 0.0 {
            Ok(rest_mid)
        } else {
            Err(CycleError::Halted(CycleOutcome::Halted(
                "NO_PRICE: missing mid price.",
            )))
        }
    }

    pub fn run_one_cycle(&mut self) -> Result<CycleOutcome, CycleError> {
        self.maybe_sync_time()?;
        self.sync_balance_if_possible();

        if self.pipeline.engine.emergency_halt.load(Ordering::Acquire) {
            return Ok(CycleOutcome::Halted("EMERGENCY_HALT: Engine halted by guard."));
        }

        if self
            .pipeline
            .engine
            .is_position_active
            .load(Ordering::Acquire)
        {
            return self.run_exit_path();
        }

        self.run_entry_path()
    }

    fn run_exit_path(&mut self) -> Result<CycleOutcome, CycleError> {
        let feed = self.live_feed()?;
        let bucket = &self.pipeline.engine.weight_bucket;
        let depth =
            self.map_exchange(self.gateway.fetch_depth_view(bucket, &self.config.trade_symbol))?;
        self.clear_error_streak();
        let mark = self.resolve_mid(&feed, depth.mid_price)?;

        let exit = run_exit_cycle(
            &self.pipeline,
            &self.gateway,
            &self.credentials,
            &self.config,
            &self.filters,
            &mut self.tracked,
            mark,
        )
        .map_err(CycleError::Exchange)?;

        if exit.exited {
            if let Some(pnl) = exit.closing_pnl_pct {
                self.journal.record_close(pnl);
            }
            Ok(CycleOutcome::PositionExited)
        } else {
            Ok(CycleOutcome::PositionActive)
        }
    }

    fn run_entry_path(&mut self) -> Result<CycleOutcome, CycleError> {
        let feed = self.live_feed()?;
        let sym = self.config.trade_symbol.clone();
        let btc_sym = self.config.btc_symbol.clone();

        let btc_chg = self.map_exchange(self.gateway.fetch_24h_change_pct(
            &self.pipeline.engine.weight_bucket,
            &btc_sym,
        ))?;
        let trade_chg = self.map_exchange(self.gateway.fetch_24h_change_pct(
            &self.pipeline.engine.weight_bucket,
            &sym,
        ))?;
        let depth = self.map_exchange(self.gateway.fetch_depth_view(
            &self.pipeline.engine.weight_bucket,
            &sym,
        ))?;
        let atr = self
            .gateway
            .fetch_atr_1h_only(&self.pipeline.engine.weight_bucket, &sym)
            .unwrap_or(0.01);
        self.clear_error_streak();

        let mid = self.resolve_mid(&feed, depth.mid_price)?;
        let cvd_scaled = feed.cvd_h1_scaled();

        let win_rate = self.journal.dynamic_win_rate();
        let win_loss_ratio = self.journal.dynamic_win_loss_ratio();
        let balance = self.pipeline.engine.balance_usd();
        let kelly = self
            .pipeline
            .engine
            .calculate_position_size(win_rate, win_loss_ratio, atr);
        let mut order_qty = contract_quantity(balance, mid, kelly, &self.filters);
        if order_qty < self.filters.min_qty {
            order_qty = self.filters.min_qty;
        }

        let btc_shock = btc_crash_shock_from_change_pct(
            btc_chg,
            self.config.btc_shock_threshold_pct,
        );

        let ts = self
            .pipeline
            .chrono
            .get_safe_exchange_timestamp()
            .map_err(|e| ExchangeError::Other(e))?;

        let market = UnifiedMarketState {
            symbol_hash: hash_symbol(&sym),
            timestamp_mono_ns: ts.saturating_mul(1_000_000),
            adjusted_price_scaled: (mid * USD_SCALE) as i64,
            cvd_h1_scaled: cvd_scaled,
            consensus_score_scaled: consensus_from_ticker_strength(trade_chg),
            lock_status: if btc_shock { 1 } else { 0 },
        };

        let input = CycleInput {
            market,
            btc_drop_shock: btc_shock,
            order_side: self.config.default_side,
            order_size: order_qty,
            bid_depth: depth.bid_notional,
            ask_depth: depth.ask_notional,
        };

        let outcome = self.pipeline.run_single_cycle(input);

        match outcome {
            CycleOutcome::AttackEngaged => self.execute_attack(order_qty, mid)?,
            CycleOutcome::ManualAlert => {
                let consensus = resolve_consensus_from_market(&market);
                let _ = send_manual_alert(&sym, consensus, trade_chg);
            }
            _ => {}
        }

        Ok(outcome)
    }

    fn execute_attack(&mut self, quantity: f64, mid: f64) -> Result<(), CycleError> {
        let executor = OrderExecutor::new(
            self.gateway.client(),
            &self.credentials,
            &self.pipeline.engine.weight_bucket,
            &self.pipeline.chrono,
            &self.filters,
        );
        let req = MarketOrderRequest {
            symbol: &self.config.trade_symbol,
            side: self.config.default_side,
            quantity,
            reduce_only: false,
        };
        match executor.place_market(req) {
            Ok(ack) => {
                eprintln!(
                    "ORDER ENTRY: id={} status={} dry_run={} qty={} kelly_w={:.2} kelly_r={:.2}",
                    ack.order_id,
                    ack.status,
                    self.credentials.dry_run,
                    quantity,
                    self.journal.dynamic_win_rate(),
                    self.journal.dynamic_win_loss_ratio()
                );
                self.pipeline
                    .engine
                    .is_position_active
                    .store(true, Ordering::SeqCst);
                self.tracked = Some(TrackedPosition::new_open(
                    mid,
                    self.filters.round_qty(quantity),
                    self.config.default_side,
                ));
                Ok(())
            }
            Err(e) => {
                self.on_exchange_error(&e);
                Err(CycleError::Exchange(e))
            }
        }
    }
}

fn hash_symbol(symbol: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    symbol.hash(&mut h);
    h.finish()
}
