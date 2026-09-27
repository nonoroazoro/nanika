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

    /// Resolve menu entries from the session-authorized target, excluding row activation.
    pub(crate) fn menu_actions(
        &self,
        request: &crate::ContextMenuRequest,
    ) -> Result<Vec<nanika_protocol::Action>, String> {
        self.authorize(request.session_id)?;
        if self.navigation.busy {
            return Err("An action is still running.".to_owned());
        }
        let (actions, primary_action_id) = match &request.target {
            crate::MenuTarget::Search {
                request_id,
                result_revision,
                extension_id,
                entry_id,
            } => {
                self.authorize_result(*request_id, *result_revision)?;
                if !self.navigation.stack.is_empty() {
                    return Err("Search changed. Reopen the menu.".to_owned());
                }
                let candidate = &self
                    .delivered
                    .as_ref()
                    .and_then(|snapshot| {
                        snapshot.results.iter().find(|result| {
                            result.candidate.extension_id() == extension_id
                                && result.candidate.entry_id() == entry_id
                        })
                    })
                    .ok_or("The result is no longer available.")?
                    .candidate;
                let primary = candidate.actions().iter().find(|action| {
                    action.id == candidate.action_id()
                        && (action.allow_default_execution || action.confirmation_title.is_some())
                });
                (
                    candidate.actions(),
                    primary.map(|action| action.id.as_str()),
                )
            }
            crate::MenuTarget::View {
                route_id,
                revision,
                item_id,
            } => {
                let route = self.navigation.authorize_route(*route_id)?;
                if route.revision != *revision {
                    return Err("The view changed. Reopen the menu.".to_owned());
                }
                let actions = match (&*route.view, item_id) {
                    (nanika_protocol::View::List { list }, Some(id)) => list
                        .sections
                        .iter()
                        .flat_map(|section| &section.items)
                        .find(|item| &item.id == id)
                        .ok_or("The item is no longer available.")?
                        .actions
                        .as_slice(),
                    (nanika_protocol::View::Detail { detail }, None) => detail.actions.as_slice(),
                    _ => return Err("The menu target is unavailable.".to_owned()),
                };
                let primary = actions.iter().find(|action| {
                    action.style == nanika_protocol::ActionStyle::Primary
                        && action.allows_invocation(nanika_protocol::ActionInvocation::Default)
                });
                (actions, primary.map(|action| action.id.as_str()))
            }
        };
        nanika_protocol::validate_actions(actions)?;
        // Keyboard and row activation own the primary action; menus expose additional actions.
        Ok(actions
            .iter()
            .filter(|action| Some(action.id.as_str()) != primary_action_id)
            .cloned()
            .collect())
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

#[cfg(test)]
#[path = "../tests/SearchSession.rs"]
mod tests;
