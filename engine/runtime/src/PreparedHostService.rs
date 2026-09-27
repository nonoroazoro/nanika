use crate::HostServiceReceipt;

/// Validated, owned input with no admitted native effect yet.
pub struct PreparedHostService<'a> {
    _submit: Box<dyn FnOnce() -> Result<HostServiceReceipt, String> + 'a>,
}

impl<'a> PreparedHostService<'a> {
    pub fn new(submit: impl FnOnce() -> Result<HostServiceReceipt, String> + 'a) -> Self {
        Self {
            _submit: Box::new(submit),
        }
    }
    pub fn admit(self) -> Result<HostServiceReceipt, String> {
        (self._submit)()
    }
}

impl std::fmt::Debug for PreparedHostService<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedHostService")
            .finish_non_exhaustive()
    }
}
