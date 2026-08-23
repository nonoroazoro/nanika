use crate::ExtensionInvocationOutput;

pub(crate) const MAX_VISIBLE_INVOCATION_OUTPUT_BYTES: usize = 16 * 1024;

/// UI-owned presentation state for one extension action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvocationPresentation {
    pub(crate) invocation_id: u64,
    pub(crate) extension_id: String,
    pub(crate) generation: u64,
    pub(crate) text: String,
    pub(crate) complete: bool,
}

impl InvocationPresentation {
    pub(crate) fn empty(invocation_id: u64, extension_id: String, generation: u64) -> Self {
        Self {
            invocation_id,
            extension_id,
            generation,
            text: String::new(),
            complete: false,
        }
    }

    pub(crate) fn from_output(output: ExtensionInvocationOutput) -> Self {
        let mut presentation =
            Self::empty(output.invocation_id, output.extension_id, output.generation);
        presentation.text = output.text;
        presentation
    }

    pub(crate) fn append(&mut self, output: ExtensionInvocationOutput) -> bool {
        if output.invocation_id < self.invocation_id {
            return false;
        }
        if output.invocation_id > self.invocation_id {
            *self = Self::from_output(output);
            return true;
        }
        self.text.push_str(&output.text);
        false
    }

    pub(crate) fn visible_text(&self) -> (&str, bool) {
        if self.text.len() <= MAX_VISIBLE_INVOCATION_OUTPUT_BYTES {
            return (&self.text, false);
        }
        let mut start = self.text.len() - MAX_VISIBLE_INVOCATION_OUTPUT_BYTES;
        while !self.text.is_char_boundary(start) {
            start += 1;
        }
        (&self.text[start..], true)
    }
}
