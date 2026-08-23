use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::{DiagnosticCode, DiagnosticRecordKey, DiagnosticSource};

/// Redaction-safe host diagnostic with an explicit technical source chain.
#[derive(Clone)]
pub struct HostDiagnostic {
    code: DiagnosticCode,
    operation: &'static str,
    user_message: String,
    safe_context: Option<String>,
    source: Option<Arc<dyn Error + Send + Sync>>,
}

const RECORD_INTERVAL: Duration = Duration::from_secs(30);
const MAX_RECORD_KEYS: usize = 256;

impl HostDiagnostic {
    pub fn new(
        code: DiagnosticCode,
        operation: &'static str,
        user_message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            operation,
            user_message: user_message.into(),
            safe_context: None,
            source: None,
        }
    }

    pub fn from_error<E>(
        code: DiagnosticCode,
        operation: &'static str,
        user_message: impl Into<String>,
        source: E,
    ) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            code,
            operation,
            user_message: user_message.into(),
            safe_context: None,
            source: Some(Arc::new(source)),
        }
    }

    pub fn from_message(
        code: DiagnosticCode,
        operation: &'static str,
        user_message: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self::from_error(
            code,
            operation,
            user_message,
            DiagnosticSource(source.into()),
        )
    }

    pub const fn code(&self) -> DiagnosticCode {
        self.code
    }

    pub const fn operation(&self) -> &'static str {
        self.operation
    }

    pub fn with_safe_context(mut self, safe_context: impl Into<String>) -> Self {
        self.safe_context = Some(safe_context.into());
        self
    }

    pub fn user_message(&self) -> &str {
        &self.user_message
    }

    pub fn record_warning(&self) {
        if !should_record(self, false) {
            return;
        }
        tracing::warn!(
            diagnostic.code = self.code.as_str(),
            diagnostic.category = self.code.category().as_str(),
            diagnostic.operation = self.operation,
            diagnostic.context = self.safe_context.as_deref().unwrap_or(""),
            "host operation failed"
        );
    }

    pub fn record_error(&self) {
        if !should_record(self, true) {
            return;
        }
        tracing::error!(
            diagnostic.code = self.code.as_str(),
            diagnostic.category = self.code.category().as_str(),
            diagnostic.operation = self.operation,
            diagnostic.context = self.safe_context.as_deref().unwrap_or(""),
            "host operation failed"
        );
    }
}

impl std::fmt::Debug for HostDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostDiagnostic")
            .field("code", &self.code)
            .field("operation", &self.operation)
            .field("safe_context", &self.safe_context)
            .field("user_message", &"<redacted>")
            .field("source", &self.source.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

impl std::fmt::Display for HostDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.user_message)
    }
}

impl Error for HostDiagnostic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

pub(crate) fn should_record(diagnostic: &HostDiagnostic, error: bool) -> bool {
    static RECENT: OnceLock<Mutex<HashMap<DiagnosticRecordKey, Instant>>> = OnceLock::new();
    let now = Instant::now();
    let recent = RECENT.get_or_init(|| Mutex::new(HashMap::new()));
    let mut recent = recent.lock().unwrap_or_else(|error| error.into_inner());
    recent.retain(|_, recorded| now.duration_since(*recorded) < RECORD_INTERVAL);
    let key = DiagnosticRecordKey::new(
        diagnostic.code,
        diagnostic.operation,
        diagnostic.safe_context.clone(),
        error,
    );
    if recent.contains_key(&key) || recent.len() >= MAX_RECORD_KEYS {
        return false;
    }
    recent.insert(key, now);
    true
}
