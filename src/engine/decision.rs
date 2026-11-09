use std::sync::atomic::Ordering;

use crate::config::{CONSENSUS_ALERT_THRESHOLD, CONSENSUS_ATTACK_THRESHOLD, USD_SCALE};
use crate::engine::signal::{
    cvd_aligned_with_side, price_from_scaled, price_lane_ready, resolve_consensus_from_market,
};
use crate::engine::state::HoorTradingEngine;
use crate::types::UnifiedMarketState;
use crate::validation::{OrderBookDepthValidator, OrderSide};

pub type DecisionOk = &'static str;
pub type DecisionError = &'static str;

pub const MANUAL_ALERT_MSG: &str =
    "MANUAL_ALERT: Consensus in 80-84 band. Telegram alert only.";

/// المسار العصبي الخطي — يستهلك `UnifiedMarketState` بالكامل.
pub fn execute_main_cycle(
    engine: &HoorTradingEngine,
    request_weight: u64,
    consensus_threshold: u32,
    consensus_alert_threshold: u32,
    btc_drop_shock: bool,
    market: &UnifiedMarketState,
    order_side: OrderSide,
    order_size: f64,
    bid_depth: f64,
    ask_depth: f64,
) -> Result<DecisionOk, DecisionError> {
    // حالة التجميد من الحقل الموحد
    if !market.is_safe_to_execute() {
        return Err("MARKET_STATE_FREEZE: lock_status blocks execution.");
    }

    if !price_lane_ready(market.adjusted_price_scaled) {
        return Ok("SCOUTING: adjusted_price not ready. Holding position.");
    }

    let _mid = price_from_scaled(market.adjusted_price_scaled);
    let _cvd = market.cvd_h1_scaled as f64 / USD_SCALE;

    // المستوى 0: ماكرو + أوزان
    if engine.emergency_halt.load(Ordering::Acquire) || btc_drop_shock {
        return Err("MACRO_EMERGENCY_HALT: Market crash or emergency lockout activated.");
    }

    if !engine.weight_bucket.can_send_request(request_weight) {
        return Err("RATE_LIMIT_GUARD: Local weight limit approaching. Request suppressed.");
    }

    if engine.is_position_active.load(Ordering::Acquire) {
        return Ok("POSITION_ACTIVE: Monitoring existing trade exits.");
    }

    let attack_th = if consensus_threshold == 0 {
        CONSENSUS_ATTACK_THRESHOLD
    } else {
        consensus_threshold
    };
    let alert_th = if consensus_alert_threshold == 0 {
        CONSENSUS_ALERT_THRESHOLD
    } else {
        consensus_alert_threshold
    };

    let consensus_score = resolve_consensus_from_market(market);

    // المستوى 1: إجماع الزخم + CVD
    if consensus_score < alert_th {
        return Ok("SCOUTING: Consensus score below threshold. Holding position.");
    }

    if consensus_score < attack_th {
        return Ok(MANUAL_ALERT_MSG);
    }

    if !cvd_aligned_with_side(order_side, market.cvd_h1_scaled) {
        return Ok("SCOUTING: CVD h1 not aligned with order side.");
    }

    // حجم مرجعي يُقارن بعمق السيولة عند السعر المعدّل
    let notional = order_size * price_from_scaled(market.adjusted_price_scaled);

    // المستوى 2: السيولة (حجم اسمي مستمد من السعر الموحد)
    OrderBookDepthValidator::validate_depth_or_reject(
        order_side,
        notional.max(order_size),
        bid_depth,
        ask_depth,
    )?;

    // المستوى 3: موافقة — القفل في runner بعد نجاح الأمر
    let _ = USD_SCALE; // ثابت دستوري مُربوط بالحالة
    Ok("ATTACK_ENGAGED: Order successfully dispatched with zero-error constraints.")
}
