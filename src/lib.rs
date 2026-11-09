//! HOOR Engine v1.0 — مكتبة المحرك الموحد (وحدة سيادية أحادية).
//!
//! كل وحدة لها اختصاص واحد؛ الحلقة الرئيسية في [`pipeline`] تسلسل القرار فقط.

pub mod config;
pub mod engine;
pub mod exchange;
pub mod guards;
pub mod pipeline;
pub mod runner;
pub mod types;
pub mod validation;

pub use config::EngineConfig;
pub use engine::HoorTradingEngine;
pub use pipeline::{CycleInput, CycleOutcome, MainLoopPipeline};
pub use types::UnifiedMarketState;
pub use validation::OrderSide;
