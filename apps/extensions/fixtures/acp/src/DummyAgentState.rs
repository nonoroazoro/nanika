use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use agent_client_protocol::schema::v1::SessionId;

#[derive(Default)]
pub(crate) struct DummyAgentState {
    next_session_id: AtomicU64,
    sessions: Mutex<HashSet<SessionId>>,
}

impl DummyAgentState {
    pub(crate) fn create_session(&self) -> SessionId {
        let sequence = self.next_session_id.fetch_add(1, Ordering::Relaxed) + 1;
        let session_id = SessionId::new(format!("hello-world-{sequence}"));
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(session_id.clone());
        session_id
    }

    pub(crate) fn contains(&self, session_id: &SessionId) -> bool {
        self.sessions
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .contains(session_id)
    }
}
