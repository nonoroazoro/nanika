pub const PROTOCOL_NAME: &str = "nanika.extension.v1";

/// Host-defined activation action for a static command contribution.
pub const COMMAND_EXECUTE_ACTION_ID: &str = "command.execute";

/// Host-defined activation action for a static view contribution.
pub const VIEW_OPEN_ACTION_ID: &str = "view.open";

/// Allocation bound for frames received from an extension process.
pub const MAX_EXTENSION_FRAME_BYTES: usize = 8 * 1024 * 1024;
