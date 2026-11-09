use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::config::USD_SCALE;

/// لقطة سوق حية — سعر + CVD ساعة من تدفق aggTrade.
#[derive(Debug)]
pub struct MarketFeed {
    mid_price_bits: AtomicU64,
    cvd_h1_scaled: AtomicI64,
    cvd_hour_bucket: AtomicU64,
    last_tick_ms: AtomicU64,
    frozen: AtomicBool,
    freeze_timeout_ms: u64,
}

impl MarketFeed {
    pub fn new(freeze_timeout_ms: u64) -> Self {
        Self {
            mid_price_bits: AtomicU64::new(0),
            cvd_h1_scaled: AtomicI64::new(0),
            cvd_hour_bucket: AtomicU64::new(0),
            last_tick_ms: AtomicU64::new(0),
            frozen: AtomicBool::new(true),
            freeze_timeout_ms,
        }
    }

    fn utc_hour_epoch() -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now / 3600
    }

    pub fn update_mid(&self, mid: f64) {
        self.mid_price_bits
            .store(mid.to_bits(), Ordering::Release);
        self.touch_alive();
    }

    /// تيك aggTrade: كمية موقعة (+ شراء عدواني، − بيع عدواني).
    pub fn apply_agg_trade(&self, qty: f64, buyer_is_maker: bool) {
        let hour = Self::utc_hour_epoch();
        let prev = self.cvd_hour_bucket.load(Ordering::Acquire);
        if prev != hour {
            self.cvd_hour_bucket.store(hour, Ordering::Release);
            self.cvd_h1_scaled.store(0, Ordering::Release);
        }
        let signed = if buyer_is_maker { -qty } else { qty };
        let delta_scaled = (signed * USD_SCALE) as i64;
        self.cvd_h1_scaled
            .fetch_add(delta_scaled, Ordering::AcqRel);
        self.touch_alive();
    }

    fn touch_alive(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.last_tick_ms.store(now, Ordering::Release);
        self.frozen.store(false, Ordering::Release);
    }

    pub fn mid_price(&self) -> Option<f64> {
        let bits = self.mid_price_bits.load(Ordering::Acquire);
        if bits == 0 {
            return None;
        }
        Some(f64::from_bits(bits))
    }

    pub fn cvd_h1_scaled(&self) -> i64 {
        let hour = Self::utc_hour_epoch();
        if self.cvd_hour_bucket.load(Ordering::Acquire) != hour {
            return 0;
        }
        self.cvd_h1_scaled.load(Ordering::Acquire)
    }

    pub fn check_freeze_watchdog(&self) -> bool {
        if self.frozen.load(Ordering::Acquire) {
            return true;
        }
        let last = self.last_tick_ms.load(Ordering::Acquire);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if last == 0 || now.saturating_sub(last) > self.freeze_timeout_ms {
            self.frozen.store(true, Ordering::Release);
            return true;
        }
        false
    }

    pub fn mark_disconnected(&self) {
        self.frozen.store(true, Ordering::Release);
    }

    /// أول تيك bookTicker حي: سعر mid والـ watchdog غير مجمّد.
    #[inline]
    pub fn is_live_ready(&self) -> bool {
        !self.frozen.load(Ordering::Acquire) && self.mid_price().is_some()
    }

    /// انتظار الجاهزية حتى `timeout` (مهلة إقلاع WebSocket).
    pub fn wait_for_live_ready(&self, timeout: Duration) -> Result<(), FeedWarmupError> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if self.is_live_ready() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err(FeedWarmupError {
            waited_ms: timeout.as_millis() as u64,
        })
    }
}

/// فشل تمهيد التدفق الحي قبل أول دورة.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedWarmupError {
    pub waited_ms: u64,
}

impl std::error::Error for FeedWarmupError {}

impl fmt::Display for FeedWarmupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FEED_WARMUP: no live bookTicker within {}ms",
            self.waited_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warmup_succeeds_after_first_mid() {
        let feed = MarketFeed::new(8_000);
        assert!(feed.wait_for_live_ready(Duration::from_millis(200)).is_err());
        feed.update_mid(50_000.0);
        assert!(feed.is_live_ready());
        assert!(feed.wait_for_live_ready(Duration::from_millis(200)).is_ok());
    }
}
