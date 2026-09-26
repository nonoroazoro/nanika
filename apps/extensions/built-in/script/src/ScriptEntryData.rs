use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptEntryData {
    pub id: String,
    pub title: String,
    pub path: PathBuf,
}
