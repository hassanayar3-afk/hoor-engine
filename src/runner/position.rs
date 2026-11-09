use crate::validation::OrderSide;

#[derive(Debug, Clone, Copy)]
pub struct TrackedPosition {
    pub entry_price: f64,
    pub initial_quantity: f64,
    pub remaining_quantity: f64,
    pub side: OrderSide,
    /// 0=دخول، 1=بعد TP1، 2=بعد TP2، 3=تريل نشط
    pub tp_stage: u8,
    pub peak_price: f64,
}

impl TrackedPosition {
    pub fn new_open(entry_price: f64, quantity: f64, side: OrderSide) -> Self {
        Self {
            entry_price,
            initial_quantity: quantity,
            remaining_quantity: quantity,
            side,
            tp_stage: 0,
            peak_price: entry_price,
        }
    }
}
