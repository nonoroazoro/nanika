/// One loaded system font face used in the UI fallback chain.
pub(crate) struct SystemFontFace {
    pub(crate) data: Vec<u8>,
    pub(crate) family: String,
    pub(crate) index: u32,
}
