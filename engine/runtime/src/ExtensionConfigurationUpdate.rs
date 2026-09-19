use nanika_protocol::ExtensionConfiguration;

/// One saved configuration snapshot waiting for a running Nanika extension.
#[derive(Debug)]
pub(crate) struct ExtensionConfigurationUpdate {
    pub(crate) request_id: String,
    pub(crate) configuration: ExtensionConfiguration,
    pub(crate) completion: Option<std::sync::mpsc::SyncSender<Result<(), String>>>,
}

impl ExtensionConfigurationUpdate {
    pub(crate) fn complete(
        self,
        extension_id: &str,
        results: &std::sync::Mutex<std::collections::VecDeque<crate::ExtensionConfigurationResult>>,
        result: Result<(), String>,
    ) {
        // A request has one consumer: the waiting Settings save or the runtime
        // update stream. Closing Settings must not accumulate duplicate reports.
        if let Some(completion) = self.completion {
            let _ = completion.send(result);
        } else {
            results
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push_back(crate::ExtensionConfigurationResult {
                    extension_id: extension_id.to_owned(),
                    request_id: self.request_id,
                    result,
                });
        }
    }
}
