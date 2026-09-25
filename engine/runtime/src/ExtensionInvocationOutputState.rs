use std::collections::VecDeque;

use crate::ExtensionInvocationOutput;

/// Action-output deltas shared by one extension worker and the UI thread.
#[derive(Debug, Default)]
pub(crate) struct ExtensionInvocationOutputState {
    pending: VecDeque<ExtensionInvocationOutput>,
    dirty: bool,
}

impl ExtensionInvocationOutputState {
    pub(crate) fn append(
        &mut self,
        instance_id: u64,
        invocation_id: u64,
        extension_id: &str,
        generation: u64,
        chunk: &str,
    ) -> bool {
        if let Some(output) = self.pending.back_mut().filter(|output| {
            output.invocation_id == invocation_id && output.instance_id == instance_id
        }) {
            output.text.push_str(chunk);
        } else {
            self.pending.push_back(ExtensionInvocationOutput {
                instance_id,
                invocation_id,
                extension_id: extension_id.to_owned(),
                generation,
                text: chunk.to_owned(),
            });
        }
        let should_notify = !self.dirty;
        self.dirty = true;
        should_notify
    }

    pub(crate) fn take_changed(&mut self) -> Option<Vec<ExtensionInvocationOutput>> {
        if !self.dirty {
            return None;
        }
        self.dirty = false;
        Some(self.pending.drain(..).collect())
    }
}
