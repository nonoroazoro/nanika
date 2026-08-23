/// Latest asynchronous storage failure retained for host diagnostics.
#[derive(Clone, Eq, PartialEq)]
pub struct SearchStorageFailure {
    sequence: u64,
    operation: &'static str,
    source: String,
}

impl SearchStorageFailure {
    pub(crate) fn new(sequence: u64, operation: &'static str, source: String) -> Self {
        Self {
            sequence,
            operation,
            source,
        }
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}
