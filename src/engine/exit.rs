use crate::validation::OrderSide;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    TakeProfit1,
    TakeProfit2,
    TakeProfit3Trail,
    StopLoss,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct PartialExitPlan {
    pub close_qty: f64,
    pub reason: ExitReason,
    pub new_stage: u8,
}

pub fn update_peak(side: OrderSide, peak: f64, mark: f64) -> f64 {
    match side {
        OrderSide::Buy => peak.max(mark),
        OrderSide::Sell => if peak == 0.0 { mark } else { peak.min(mark) },
    }
}

/// TP1/TP2 جزئيان + TP3 trailing.
pub fn evaluate_staged_exit(
    side: OrderSide,
    entry: f64,
    mark: f64,
    stage: u8,
    peak: f64,
    remaining: f64,
    tp1_pct: f64,
    tp2_pct: f64,
    tp3_trail_pct: f64,
    sl_pct: f64,
    leg_fraction: f64,
) -> (PartialExitPlan, f64) {
    let peak = update_peak(side, peak, mark);

    let pnl_pct = match side {
        OrderSide::Buy => (mark - entry) / entry * 100.0,
        OrderSide::Sell => (entry - mark) / entry * 100.0,
    };

    if pnl_pct <= -sl_pct {
        return (
            PartialExitPlan {
                close_qty: remaining,
                reason: ExitReason::StopLoss,
                new_stage: 0,
            },
            peak,
        );
    }

    match stage {
        0 if pnl_pct >= tp1_pct => {
            let q = remaining * leg_fraction;
            (
                PartialExitPlan {
                    close_qty: q,
                    reason: ExitReason::TakeProfit1,
                    new_stage: 1,
                },
                peak,
            )
        }
        1 if pnl_pct >= tp2_pct => {
            let q = remaining * leg_fraction;
            (
                PartialExitPlan {
                    close_qty: q,
                    reason: ExitReason::TakeProfit2,
                    new_stage: 2,
                },
                peak,
            )
        }
        2 | 3 => {
            let trail_hit = match side {
                OrderSide::Buy => mark <= peak * (1.0 - tp3_trail_pct / 100.0),
                OrderSide::Sell => mark >= peak * (1.0 + tp3_trail_pct / 100.0),
            };
            if trail_hit {
                (
                    PartialExitPlan {
                        close_qty: remaining,
                        reason: ExitReason::TakeProfit3Trail,
                        new_stage: 3,
                    },
                    peak,
                )
            } else {
                (
                    PartialExitPlan {
                        close_qty: 0.0,
                        reason: ExitReason::None,
                        new_stage: if stage == 2 { 3 } else { 3 },
                    },
                    peak,
                )
            }
        }
        _ => (
            PartialExitPlan {
                close_qty: 0.0,
                reason: ExitReason::None,
                new_stage: stage,
            },
            peak,
        ),
    }
}

pub fn closing_side(open_side: OrderSide) -> OrderSide {
    match open_side {
        OrderSide::Buy => OrderSide::Sell,
        OrderSide::Sell => OrderSide::Buy,
    }
}
