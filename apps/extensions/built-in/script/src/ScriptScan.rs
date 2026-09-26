use nanika_extension_script::ScriptConfig;

pub(crate) struct ScriptScan {
    pub(crate) request_id: Option<String>,
    pub(crate) generation: u64,
    pub(crate) config: ScriptConfig,
    pub(crate) configuration: bool,
}
