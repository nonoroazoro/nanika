pub(super) struct ShellLinkMetadata {
    pub(super) target: String,
    pub(super) arguments: Option<String>,
    pub(super) working_directory: Option<String>,
    pub(super) icon_source: Option<String>,
    pub(super) icon_index: i32,
}
