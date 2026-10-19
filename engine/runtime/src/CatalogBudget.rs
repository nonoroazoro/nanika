use std::io::{self, Write};

/// Count encoded input without allocating another payload or preparing search indexes.
/// Both bytes and batches are charged, so empty or duplicate batches cannot evade admission.
pub(crate) struct CatalogBudget {
    _remaining_bytes: usize,
    _remaining_batches: usize
}

impl Default for CatalogBudget {
    fn default() -> Self {
        Self::new(64 * 1024 * 1024, 4096)
    }
}

impl CatalogBudget {
    pub(crate) fn new(bytes: usize, batches: usize) -> Self {
        Self {
            _remaining_bytes: bytes,
            _remaining_batches: batches
        }
    }

    pub(crate) fn admit(&mut self, batch: &nanika_protocol::CatalogBatch) -> Result<(), String> {
        self._remaining_batches = self
            ._remaining_batches
            .checked_sub(1)
            .ok_or("catalog transaction exceeds the batch budget")?;
        serde_json::to_writer(self, batch)
            .map_err(|error| format!("catalog transaction exceeds the byte budget: {error}"))
    }
}

impl Write for CatalogBudget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self._remaining_bytes = self
            ._remaining_bytes
            .checked_sub(bytes.len())
            .ok_or_else(|| io::Error::other("catalog staging budget exhausted"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
