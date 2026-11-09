#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    Buy,
    Sell,
}

/// فحص عمق دفتر الأوامر قبل التنفيذ.
pub struct OrderBookDepthValidator;

impl OrderBookDepthValidator {
    const MIN_LIQUIDITY_MULTIPLIER: f64 = 5.0;

    pub fn validate_depth_or_reject(
        order_side: OrderSide,
        order_size: f64,
        available_bid_depth_sum: f64,
        available_ask_depth_sum: f64,
    ) -> Result<(), &'static str> {
        let required_depth = order_size * Self::MIN_LIQUIDITY_MULTIPLIER;
        let relevant_depth = match order_side {
            OrderSide::Buy => available_ask_depth_sum,
            OrderSide::Sell => available_bid_depth_sum,
        };

        if relevant_depth < required_depth {
            return Err(
                "LIQUIDITY_VACUUM_PANIC: Target Order Book Depth is below 5x required multiplier.",
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_thin_book() {
        let err = OrderBookDepthValidator::validate_depth_or_reject(
            OrderSide::Buy,
            10.0,
            100.0,
            40.0,
        );
        assert!(err.is_err());
    }

    #[test]
    fn accepts_deep_book() {
        assert!(OrderBookDepthValidator::validate_depth_or_reject(
            OrderSide::Buy,
            10.0,
            100.0,
            60.0,
        )
        .is_ok());
    }
}
