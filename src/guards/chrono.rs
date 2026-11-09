use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;

/// درع التوقيت — مزامنة وقت باينانس دون انتهاك النافذة الزمنية للتوقيع.
pub struct ChronoGuard {
    server_time_offset_ms: Arc<AtomicI64>,
    last_recalibration_instant: Arc<Mutex<Instant>>,
    recalibration_interval: Duration,
}

impl ChronoGuard {
    pub fn new(offset: Arc<AtomicI64>) -> Self {
        Self {
            server_time_offset_ms: offset,
            last_recalibration_instant: Arc::new(Mutex::new(Instant::now())),
            recalibration_interval: Duration::from_secs(60),
        }
    }

    pub fn recalibration_interval(&self) -> Duration {
        self.recalibration_interval
    }

    pub fn update_offset(&self, binance_server_time_ms: u64) {
        let system_now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if system_now_ms > 0 {
            let new_offset = binance_server_time_ms as i64 - system_now_ms as i64;
            self.server_time_offset_ms
                .store(new_offset, Ordering::Release);
            *self.last_recalibration_instant.lock() = Instant::now();
        }
    }

    pub fn get_safe_exchange_timestamp(&self) -> Result<u64, &'static str> {
        let system_now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .map_err(|_| "SYSTEM_CLOCK_ERROR: Clock moved backwards")?;

        let offset = self.server_time_offset_ms.load(Ordering::Acquire);
        let safe_timestamp = if offset >= 0 {
            system_now_ms.saturating_add(offset as u64)
        } else {
            system_now_ms.saturating_sub(offset.unsigned_abs())
        };

        Ok(safe_timestamp)
    }
}
