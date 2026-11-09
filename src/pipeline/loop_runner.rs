use std::sync::Arc;

use std::sync::atomic::AtomicI64;



use crate::config::EngineConfig;

use crate::engine::{execute_main_cycle, HoorTradingEngine};

use crate::guards::ChronoGuard;



use super::{CycleInput, CycleOutcome};



/// Sovereign single cycle: safe time, market state, linear decision.

pub struct MainLoopPipeline {

    pub engine: HoorTradingEngine,

    pub chrono: ChronoGuard,

    pub config: EngineConfig,

}



impl MainLoopPipeline {

    pub fn new(config: EngineConfig) -> Self {

        let offset = Arc::new(AtomicI64::new(0));

        Self {

            engine: HoorTradingEngine::new(config.initial_balance_usd),

            chrono: ChronoGuard::new(offset),

            config,

        }

    }



    pub fn run_single_cycle(&self, input: CycleInput) -> CycleOutcome {

        let _exchange_ts = match self.chrono.get_safe_exchange_timestamp() {

            Ok(ts) => ts,

            Err(e) => return CycleOutcome::Halted(e),

        };



        let result = execute_main_cycle(

            &self.engine,

            self.config.request_weight_per_cycle,

            self.config.consensus_threshold,

            self.config.consensus_alert_threshold,

            input.btc_drop_shock,

            &input.market,

            input.order_side,

            input.order_size,

            input.bid_depth,

            input.ask_depth,

        );



        let _ = _exchange_ts;

        CycleOutcome::from_pipeline_result(result)

    }

}


