/// Technical source retained without becoming the default diagnostic display.
#[derive(Debug)]
pub(crate) struct DiagnosticSource(pub(crate) String);

impl std::fmt::Display for DiagnosticSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for DiagnosticSource {}
