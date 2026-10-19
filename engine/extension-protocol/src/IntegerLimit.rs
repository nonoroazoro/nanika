use serde::{Deserialize, Serialize};

/// A remembered finite value and an independent unlimited state.
/// Both fields are persisted together; unlimited never discards the finite value.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct IntegerLimit {
    pub unlimited: bool,
    pub value: i64
}

impl IntegerLimit {
    pub fn effective(self) -> Option<i64> {
        (!self.unlimited).then_some(self.value)
    }
}
