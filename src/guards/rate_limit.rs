use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{WEIGHT_CEILING, WEIGHT_RESET_WINDOW_MS};

/// عداد وزن الطلبات المحلي — وقاية مسبقة من 429/418.
pub struct LocalWeightBucket {
    pub current_weight: AtomicU64,
    pub last_reset_timestamp_ms: AtomicU64,
    ceiling: u64,
    reset_window_ms: u64,
}

impl LocalWeightBucket {
    pub fn new() -> Self {
        Self::with_limits(WEIGHT_CEILING, WEIGHT_RESET_WINDOW_MS)
    }

    pub fn with_limits(ceiling: u64, reset_window_ms: u64) -> Self {
        let now = Self::now_ms();
        Self {
            current_weight: AtomicU64::new(0),
            last_reset_timestamp_ms: AtomicU64::new(now),
            ceiling,
            reset_window_ms,
        }
    }

    #[inline]
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    #[inline(always)]
    pub fn can_send_request(&self, request_weight: u64) -> bool {
        let now = Self::now_ms();
        let last_reset = self.last_reset_timestamp_ms.load(Ordering::Acquire);

        if now.saturating_sub(last_reset) >= self.reset_window_ms {
            self.current_weight.store(0, Ordering::Release);
            self.last_reset_timestamp_ms.store(now, Ordering::Release);
        }

        let current = self.current_weight.load(Ordering::Acquire);
        if current.saturating_add(request_weight) > self.ceiling {
            return false;
        }

        self.current_weight
            .fetch_add(request_weight, Ordering::SeqCst);
        true
    }
}

impl Default for LocalWeightBucket {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respects_ceiling() {
        let bucket = LocalWeightBucket::with_limits(10, WEIGHT_RESET_WINDOW_MS);
        assert!(bucket.can_send_request(5));
        assert!(bucket.can_send_request(5));
        assert!(!bucket.can_send_request(1));
    }
}
