//! Built-in configured script extension.

#[path = "RuntimePaths.rs"]
mod runtime_paths;
#[path = "ScriptConfig.rs"]
mod script_config;
#[path = "ScriptEntry.rs"]
mod script_entry;

pub use runtime_paths::*;
pub use script_config::*;
pub use script_entry::*;

pub const EXTENSION_ID: &str = "com.nanika.script";
pub const RUN_ACTION_ID: &str = "script.run";
