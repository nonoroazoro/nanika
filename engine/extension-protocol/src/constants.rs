pub const PROTOCOL_NAME: &str = "nanika.extension.v1";

/// Host-defined activation action for a static command contribution.
pub const COMMAND_EXECUTE_ACTION_ID: &str = "command.execute";

/// Host-defined activation action for a static view contribution.
pub const VIEW_OPEN_ACTION_ID: &str = "view.open";

/// Unicode scalar values requested per progressive plain-text batch.
pub const DETAIL_TEXT_BATCH_CHARS: usize = 16_384;

/// Resident plain-text preview budget, including chunks without visible glyphs.
/// Full payload storage and copy actions remain the extension's responsibility.
pub const MAX_DETAIL_TEXT_CHUNKS: usize = 64;

/// A delivery bound, never a limit on searchable or retained items.
pub const MAX_VIEW_ITEMS: usize = 500;
