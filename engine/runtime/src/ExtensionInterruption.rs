#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionInterruption {
    None,
    Cancel,
    Terminate,
}
