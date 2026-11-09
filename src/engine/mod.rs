mod decision;
mod exit;
mod journal;
mod kelly;
mod signal;
mod state;
mod tp_recovery;

pub use decision::{execute_main_cycle, DecisionError, DecisionOk, MANUAL_ALERT_MSG};
pub use exit::{closing_side, evaluate_staged_exit, ExitReason, PartialExitPlan};
pub use journal::TradeJournal;
pub use signal::resolve_consensus_from_market;
pub use state::HoorTradingEngine;
pub use tp_recovery::infer_tp_stage;
