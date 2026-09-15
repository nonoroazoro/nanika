use std::sync::Arc;
use std::time::Instant;

use crate::{RootSearchSnapshot, SearchPhase};

/// One WebView lifetime, with at most one unacknowledged Channel message.
pub(crate) struct SearchSession {
    pub(crate) navigation: crate::NavigationState,
    pub(crate) delivered_navigation_revision: u64,
    pub(crate) id: u64,
    pub(crate) request_id: u64,
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) updates: tauri::ipc::Channel<RootSearchSnapshot>,
    pub(crate) revision: u64,
    pub(crate) in_flight: Option<(u64, Instant)>,
    pub(crate) delivered: Option<Arc<nanika_search::SearchSnapshot>>,
    pub(crate) phase: Option<SearchPhase>,
    pub(crate) error: Option<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) transport_error: Option<String>,
}

impl SearchSession {
    pub(crate) fn new(id: u64, updates: tauri::ipc::Channel<RootSearchSnapshot>) -> Self {
        Self {
            navigation: crate::NavigationState::default(),
            delivered_navigation_revision: 0,
            id,
            request_id: 0,
            generation: 0,
            query: String::new(),
            updates,
            revision: 0,
            in_flight: None,
            delivered: None,
            phase: None,
            error: None,
            warnings: Vec::new(),
            transport_error: None,
        }
    }

    pub(crate) fn begin_refresh(&mut self, session_id: u64) -> Result<(), String> {
        self.authorize(session_id)?;
        if !self.navigation.stack.is_empty() {
            return Err("Refresh is available only in Root Search.".to_owned());
        }
        self.navigation.begin()
    }

    pub(crate) fn authorize(&self, session_id: u64) -> Result<(), String> {
        if self.id != session_id {
            return Err("The window session has expired. Reload the window.".to_owned());
        }
        if let Some(error) = &self.transport_error {
            return Err(error.clone());
        }
        Ok(())
    }
}
