use std::io::Write;

/// Writer that accepts a bounded number of bytes and drops later output.
pub(crate) struct BoundedLogWriter<W> {
    inner: W,
    remaining: u64,
}

impl<W> BoundedLogWriter<W> {
    pub(crate) const fn new(inner: W, remaining: u64) -> Self {
        Self { inner, remaining }
    }

    #[cfg(test)]
    pub(crate) fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for BoundedLogWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let accepted = usize::try_from(self.remaining)
            .unwrap_or(usize::MAX)
            .min(buffer.len());
        if accepted > 0 {
            self.inner.write_all(&buffer[..accepted])?;
            self.remaining = self.remaining.saturating_sub(accepted as u64);
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
