//! Built-in deterministic calculator extension.

#[path = "CalculatorEngine.rs"]
mod calculator_engine;
pub use calculator_engine::*;

pub const EXTENSION_ID: &str = "com.nanika.calculator";
pub const COPY_ACTION_ID: &str = "calculator.copy";

#[path = "QueryInterrupt.rs"]
mod query_interrupt;
use query_interrupt::QueryInterrupt;
