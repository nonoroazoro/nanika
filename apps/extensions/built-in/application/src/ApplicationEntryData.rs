use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationEntryData {
    pub entry_id: String,
    pub source_key: String,
    pub display_name: String,
    pub normalized_name: String,
    pub normalized_tokens: String,
    pub launch_kind: String,
    /// Native activation path, preserving the original spelling of Shell Links.
    pub target_path: String,
    pub arguments_json: String,
    pub icon_key: String,
    pub(crate) icon_source: Option<PathBuf>,
    pub(crate) icon_index: i32,
    pub(crate) priority: usize,
}
