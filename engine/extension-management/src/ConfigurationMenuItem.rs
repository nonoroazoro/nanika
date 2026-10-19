use serde::{Deserialize, Serialize};

/// A shortcut references its Settings definition; it never supplies a second value or handler.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ConfigurationMenuItem {
    Toggle { key: String },
    Action { id: String }
}
