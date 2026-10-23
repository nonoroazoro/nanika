use std::fmt;

pub(crate) struct BoundedOutput {
    pub(crate) text: String,
    pub(crate) maximum_bytes: usize
}

impl fmt::Write for BoundedOutput {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if value.len() > self.maximum_bytes.saturating_sub(self.text.len()) {
            return Err(fmt::Error);
        }
        self.text.push_str(value);
        Ok(())
    }
}
