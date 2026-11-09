use std::sync::atomic::Ordering;

use crate::engine::infer_tp_stage;
use crate::exchange::parse_symbol_filters;
use crate::exchange::rest::has_open_orders;
use crate::exchange::ExchangeError;
use crate::runner::position::TrackedPosition;
use crate::validation::OrderSide;

use super::HoorRunner;

impl HoorRunner {
  pub fn startup_recovery(&mut self) -> Result<(), ExchangeError> {
        let bucket = &self.pipeline.engine.weight_bucket;
        let sym = &self.config.trade_symbol;

        let json = {
            if !bucket.can_send_request(crate::exchange::binance_config::request_weight::EXCHANGE_INFO)
            {
                return Err(ExchangeError::WeightDenied {
                    weight: crate::exchange::binance_config::request_weight::EXCHANGE_INFO,
                });
            }
            self.gateway.client().get_exchange_info_json()?
        };
        self.filters = parse_symbol_filters(&json, sym)?;
        self.config.qty_step = self.filters.step_size;

        if !self.credentials.has_api_keys() {
            return Ok(());
        }

        if let Some(pos) = self.gateway.fetch_open_position(
            &self.credentials,
            bucket,
            &self.pipeline.chrono,
            sym,
        )? {
            let side = if pos.position_amt > 0.0 {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };
            let mark = pos.mark_price.max(pos.entry_price);
            let mut t = TrackedPosition::new_open(pos.entry_price, pos.position_amt.abs(), side);
            t.tp_stage = infer_tp_stage(
                side,
                pos.entry_price,
                mark,
                self.config.tp1_pct,
                self.config.tp2_pct,
            );
            t.peak_price = mark;
            let stage = t.tp_stage;
            self.tracked = Some(t);
            self.pipeline
                .engine
                .is_position_active
                .store(true, Ordering::SeqCst);
            eprintln!(
                "STARTUP: position recovered qty={} tp_stage={}",
                pos.position_amt, stage
            );
        }

        if has_open_orders(
            self.gateway.client(),
            &self.credentials,
            bucket,
            &self.pipeline.chrono,
            sym,
        )? {
            self.pipeline
                .engine
                .is_position_active
                .store(true, Ordering::SeqCst);
            eprintln!("STARTUP: open orders detected — position lock engaged");
        }

        Ok(())
    }
}
