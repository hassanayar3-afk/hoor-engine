/// حالة السوق الموحدة (تخطيط Zero-Copy عبر `repr(C)`).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UnifiedMarketState {
    pub symbol_hash: u64,
    pub timestamp_mono_ns: u64,
    /// السعر المعدّل × 10⁸
    pub adjusted_price_scaled: i64,
    /// CVD الساعة × 10⁸
    pub cvd_h1_scaled: i64,
    /// طبقة التيكر 24س فقط؛ الإجماع الكامل في `engine::signal::resolve_consensus_from_market`.
    pub consensus_score_scaled: u32,
    /// 0 = طبيعي، 1 = تجميد طوارئ
    pub lock_status: u8,
}

impl UnifiedMarketState {
    #[inline(always)]
    pub fn is_safe_to_execute(&self) -> bool {
        self.lock_status == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lock_blocks_execution() {
        let mut s = UnifiedMarketState {
            symbol_hash: 0,
            timestamp_mono_ns: 0,
            adjusted_price_scaled: 0,
            cvd_h1_scaled: 0,
            consensus_score_scaled: 0,
            lock_status: 1,
        };
        assert!(!s.is_safe_to_execute());
        s.lock_status = 0;
        assert!(s.is_safe_to_execute());
    }
}
