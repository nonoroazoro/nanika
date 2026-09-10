#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionInterruption {
    None,
    Cancel,
    Terminate,
}
