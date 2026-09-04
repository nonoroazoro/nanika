//! Built-in deterministic calculator extension.

#[path = "CalculatorEngine.rs"]
mod calculator_engine;
#[path = "DeadlineInterrupt.rs"]
mod deadline_interrupt;

pub use calculator_engine::*;
pub(crate) use deadline_interrupt::*;

pub const EXTENSION_ID: &str = "com.nanika.calculator";
pub const COPY_ACTION_ID: &str = "calculator.copy";
