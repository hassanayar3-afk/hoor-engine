use crate::types::UnifiedMarketState;
use crate::validation::OrderSide;

/// مدخلات دورة واحدة من الحلقة الرئيسية (لا تشمل اتصال الشبكة بعد).
#[derive(Debug, Clone, Copy)]
pub struct CycleInput {
    pub market: UnifiedMarketState,
    pub btc_drop_shock: bool,
    pub order_side: OrderSide,
    pub order_size: f64,
    pub bid_depth: f64,
    pub ask_depth: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    AttackEngaged,
    Scouting,
    PositionActive,
    PositionExited,
    ManualAlert,
    Rejected(&'static str),
    Halted(&'static str),
}

impl CycleOutcome {
    pub fn from_pipeline_result(result: Result<&'static str, &'static str>) -> Self {
        match result {
            Ok("ATTACK_ENGAGED: Order successfully dispatched with zero-error constraints.") => {
                CycleOutcome::AttackEngaged
            }
            Ok("SCOUTING: Consensus score below threshold. Holding position.") => {
                CycleOutcome::Scouting
            }
            Ok("POSITION_ACTIVE: Monitoring existing trade exits.") => CycleOutcome::PositionActive,
            Ok(crate::engine::MANUAL_ALERT_MSG) => CycleOutcome::ManualAlert,
            Ok(_) => CycleOutcome::Scouting,
            Err(e) if e.starts_with("MACRO_EMERGENCY") => CycleOutcome::Halted(e),
            Err(e) => CycleOutcome::Rejected(e),
        }
    }
}
