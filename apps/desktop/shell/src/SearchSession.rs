use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{RootSearchSnapshot, SearchPhase};

/// One WebView lifetime, with at most one unacknowledged Channel message.
pub(crate) struct SearchSession {
    pub(crate) view_operation_lock: Arc<Mutex<()>>,
    pub(crate) navigation: crate::NavigationState,
    pub(crate) delivered_navigation_revision: u64,
    pub(crate) delivered_route: Option<(u64, u64)>,
    pub(crate) id: u64,
    pub(crate) request_id: u64,
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) updates: tauri::ipc::Channel<RootSearchSnapshot>,
    pub(crate) revision: u64,
    pub(crate) range_id: u64,
    pub(crate) result_revision: u64,
    pub(crate) result_range: (usize, usize),
    pub(crate) delivered_range: Option<(usize, usize)>,
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
            view_operation_lock: Arc::new(Mutex::new(())),
            navigation: crate::NavigationState::default(),
            delivered_navigation_revision: 0,
            delivered_route: None,
            id,
            request_id: 0,
            generation: 0,
            query: String::new(),
            updates,
            revision: 0,
            result_revision: 0,
            range_id: 0,
            result_range: (0, 64),
            delivered_range: None,
            in_flight: None,
            delivered: None,
            phase: None,
            error: None,
            warnings: Vec::new(),
            transport_error: None,
        }
    }

    pub(crate) fn request_range(
        &mut self,
        request: crate::ReadResultsRequest,
    ) -> Result<(), String> {
        self.authorize(request.session_id)?;
        if request.count == 0
            || request.range_id == 0
            || request.range_id > 9_007_199_254_740_991
            || request.offset.checked_add(request.count).is_none()
        {
            return Err("Invalid result range.".to_owned());
        }
        // Scroll requests can arrive out of order or after their ranking was replaced.
        if request.request_id != self.request_id
            || request.result_revision != self.result_revision
            || request.range_id <= self.range_id
        {
            return Ok(());
        }
        self.range_id = request.range_id;
        self.result_range = (request.offset, request.count);
        Ok(())
    }

    pub(crate) fn authorize_result(
        &self,
        request_id: u64,
        result_revision: u64,
    ) -> Result<(), String> {
        if request_id != self.request_id || result_revision != self.result_revision {
            return Err("Search changed. Select a current result.".to_owned());
        }
        Ok(())
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
