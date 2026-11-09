use crate::engine::{closing_side, evaluate_staged_exit, ExitReason};
use crate::exchange::rest::has_open_orders;
use crate::exchange::rest::{MarketOrderRequest, OrderExecutor};
use crate::exchange::{BinanceGateway, Credentials, ExchangeError, SymbolFilters};
use crate::pipeline::MainLoopPipeline;
use crate::validation::OrderSide;

use super::position::TrackedPosition;
use super::RunnerConfig;

pub struct ExitCycleResult {
    pub exited: bool,
    pub partial: bool,
    pub reason: Option<ExitReason>,
    pub closing_pnl_pct: Option<f64>,
}

/// أوامر معلقة أو مركز — يُحافظ على قفل الصفقة.
pub fn exchange_requires_position_lock(
    pipeline: &MainLoopPipeline,
    gateway: &BinanceGateway,
    creds: &Credentials,
    config: &RunnerConfig,
) -> Result<bool, ExchangeError> {
    if !creds.has_api_keys() {
        return Ok(false);
    }
    let bucket = &pipeline.engine.weight_bucket;
    if gateway.fetch_open_position(creds, bucket, &pipeline.chrono, &config.trade_symbol)?
        .is_some()
    {
        return Ok(true);
    }
    Ok(has_open_orders(
        gateway.client(),
        creds,
        bucket,
        &pipeline.chrono,
        &config.trade_symbol,
    )?)
}

pub fn run_exit_cycle(
    pipeline: &MainLoopPipeline,
    gateway: &BinanceGateway,
    creds: &Credentials,
    config: &RunnerConfig,
    filters: &SymbolFilters,
    tracked: &mut Option<TrackedPosition>,
    mark_price: f64,
) -> Result<ExitCycleResult, ExchangeError> {
    let bucket = &pipeline.engine.weight_bucket;

    if creds.has_api_keys() && !creds.dry_run {
        sync_tracked_from_exchange(pipeline, gateway, creds, config, tracked)?;
    }

    let Some(pos) = tracked.as_mut() else {
        if exchange_requires_position_lock(pipeline, gateway, creds, config)? {
            pipeline
                .engine
                .is_position_active
                .store(true, std::sync::atomic::Ordering::SeqCst);
            return Ok(ExitCycleResult {
                exited: false,
                partial: false,
                reason: None,
                closing_pnl_pct: None,
            });
        }
        pipeline.engine.release_position_lock();
        return Ok(ExitCycleResult {
            exited: false,
            partial: false,
            reason: None,
            closing_pnl_pct: None,
        });
    };

    let (plan, new_peak) = evaluate_staged_exit(
        pos.side,
        pos.entry_price,
        mark_price,
        pos.tp_stage,
        pos.peak_price,
        pos.remaining_quantity,
        config.tp1_pct,
        config.tp2_pct,
        config.tp3_trail_pct,
        config.stop_loss_pct,
        config.tp_leg_fraction,
    );
    pos.peak_price = new_peak;

    if plan.close_qty <= 0.0 || plan.reason == ExitReason::None {
        return Ok(ExitCycleResult {
            exited: false,
            partial: false,
            reason: Some(ExitReason::None),
            closing_pnl_pct: None,
        });
    }

    let planned = filters.round_qty(plan.close_qty.min(pos.remaining_quantity));
    let close_qty = match resolve_close_qty_notional(
        planned,
        pos.remaining_quantity,
        mark_price,
        filters,
        plan.reason,
    )? {
        Some(q) => q,
        None => {
            return Ok(ExitCycleResult {
                exited: false,
                partial: false,
                reason: Some(ExitReason::None),
                closing_pnl_pct: None,
            });
        }
    };

    let executor = OrderExecutor::new(
        gateway.client(),
        creds,
        bucket,
        &pipeline.chrono,
        filters,
    );
    let close_side = closing_side(pos.side);
    executor.place_market(MarketOrderRequest {
        symbol: &config.trade_symbol,
        side: close_side,
        quantity: close_qty,
        reduce_only: true,
    })?;

    pos.remaining_quantity = filters.round_qty(pos.remaining_quantity - close_qty);
    pos.tp_stage = plan.new_stage;

    let fully_closed = pos.remaining_quantity < filters.min_qty
        || plan.reason == ExitReason::StopLoss
        || plan.reason == ExitReason::TakeProfit3Trail;

    let pnl_pct = pnl_from_entry(pos.side, pos.entry_price, mark_price);

    if fully_closed {
        if !exchange_requires_position_lock(pipeline, gateway, creds, config)? {
            pipeline.engine.release_position_lock();
            *tracked = None;
        }
        return Ok(ExitCycleResult {
            exited: true,
            partial: plan.reason != ExitReason::StopLoss,
            reason: Some(plan.reason),
            closing_pnl_pct: Some(pnl_pct),
        });
    }

    Ok(ExitCycleResult {
        exited: false,
        partial: true,
        reason: Some(plan.reason),
        closing_pnl_pct: None,
    })
}

fn pnl_from_entry(side: crate::validation::OrderSide, entry: f64, mark: f64) -> f64 {
    match side {
        OrderSide::Buy => (mark - entry) / entry * 100.0,
        OrderSide::Sell => (entry - mark) / entry * 100.0,
    }
}

/// يضمن احترام `minNotional`؛ يُرجع `None` لتأجيل جزء TP الصغير.
fn resolve_close_qty_notional(
    planned_qty: f64,
    remaining_qty: f64,
    mark_price: f64,
    filters: &SymbolFilters,
    reason: ExitReason,
) -> Result<Option<f64>, ExchangeError> {
    if mark_price <= 0.0 {
        return Err(ExchangeError::Other("MARK_PRICE_ZERO"));
    }

    let notional = |q: f64| q * mark_price;

    if notional(planned_qty) >= filters.min_notional {
        return Ok(Some(planned_qty));
    }

    match reason {
        ExitReason::StopLoss | ExitReason::TakeProfit3Trail => {
            if notional(remaining_qty) >= filters.min_notional {
                return Ok(Some(filters.round_qty(remaining_qty)));
            }
            Err(ExchangeError::Other("EXIT_BELOW_MIN_NOTIONAL"))
        }
        ExitReason::TakeProfit1 | ExitReason::TakeProfit2 => {
            if notional(remaining_qty) >= filters.min_notional {
                Ok(None)
            } else {
                Err(ExchangeError::Other("PARTIAL_BELOW_MIN_NOTIONAL"))
            }
        }
        ExitReason::None => Ok(None),
    }
}

fn sync_tracked_from_exchange(
    pipeline: &MainLoopPipeline,
    gateway: &BinanceGateway,
    creds: &Credentials,
    config: &RunnerConfig,
    tracked: &mut Option<TrackedPosition>,
) -> Result<(), ExchangeError> {
    if let Some(pos) = gateway.fetch_open_position(
        creds,
        &pipeline.engine.weight_bucket,
        &pipeline.chrono,
        &config.trade_symbol,
    )? {
        let side = if pos.position_amt > 0.0 {
            OrderSide::Buy
        } else {
            OrderSide::Sell
        };
        let qty = pos.position_amt.abs();
        if let Some(t) = tracked.as_mut() {
            t.remaining_quantity = qty;
            t.entry_price = pos.entry_price;
        } else {
            *tracked = Some(TrackedPosition::new_open(pos.entry_price, qty, side));
        }
        pipeline
            .engine
            .is_position_active
            .store(true, std::sync::atomic::Ordering::SeqCst);
    } else if tracked.is_some() {
        if !exchange_requires_position_lock(pipeline, gateway, creds, config)? {
            pipeline.engine.release_position_lock();
            *tracked = None;
        }
    } else if exchange_requires_position_lock(pipeline, gateway, creds, config)? {
        pipeline
            .engine
            .is_position_active
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ExitReason;
    use crate::exchange::SymbolFilters;

    fn filters() -> SymbolFilters {
        SymbolFilters {
            step_size: 0.001,
            min_qty: 0.001,
            min_notional: 100.0,
            tick_size: 0.01,
            qty_precision: 3,
        }
    }

    #[test]
    fn defers_small_tp_leg() {
        let q = resolve_close_qty_notional(
            0.001,
            1.0,
            50_000.0,
            &filters(),
            ExitReason::TakeProfit1,
        )
        .unwrap();
        assert!(q.is_none());
    }

    #[test]
    fn stop_closes_full_when_leg_too_small() {
        let q = resolve_close_qty_notional(
            0.001,
            0.01,
            50_000.0,
            &filters(),
            ExitReason::StopLoss,
        )
        .unwrap();
        assert!(q.is_some());
    }
}
