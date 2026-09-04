/// Result of one application discovery generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanReport {
    pub generation: u64,
    pub discovered: usize,
    pub warnings: usize,
    pub complete: bool,
    pub cancelled: bool,
}
