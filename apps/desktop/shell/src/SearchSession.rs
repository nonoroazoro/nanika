use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{RootSearchSnapshot, SearchPhase};
use nanika_host::{OperationError, OperationErrorCode as Code};

/// One WebView lifetime, with at most one unacknowledged Channel message.
pub(crate) struct SearchSession {
    pub(crate) text_instances: std::collections::HashMap<String, u64>,
    pub(crate) text_invalidations:
        std::collections::HashMap<(String, u64), crate::TextInvalidationNotice>,
    pub(crate) view_operation_lock: Arc<Mutex<()>>,
    pub(crate) navigation: crate::NavigationState,
    pub(crate) delivered_localization_revision: u64,
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
    pub(crate) error: Option<nanika_host::OperationError>,
    pub(crate) warnings: Vec<nanika_host::RuntimeWarning>,
    pub(crate) transport_error: Option<String>
}

impl SearchSession {
    pub(crate) fn new(id: u64, updates: tauri::ipc::Channel<RootSearchSnapshot>) -> Self {
        Self {
            text_instances: Default::default(),
            text_invalidations: Default::default(),
            view_operation_lock: Arc::new(Mutex::new(())),
            navigation: crate::NavigationState::default(),
            delivered_navigation_revision: 0,
            delivered_localization_revision: 0,
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
            transport_error: None
        }
    }

    pub(crate) fn configuration_menu_route(
        &self,
        request: &crate::ConfigurationMenuRequest
    ) -> Result<&crate::ExtensionViewSnapshot, OperationError> {
        self.authorize(request.session_id)?;
        if self.navigation.busy {
            return Err(OperationError::new(Code::OperationPending));
        }
        let route = self
            .navigation
            .authorize_route(request.route_id)
            .map_err(|_| OperationError::new(Code::ViewChanged))?;
        if route.instance_id != request.instance_id {
            return Err(OperationError::new(Code::ViewChanged));
        }
        Ok(route)
    }

    /// Resolve menu entries from the session-authorized target, excluding row activation.
    pub(crate) fn menu_actions(
        &self,
        request: &crate::ContextMenuRequest
    ) -> Result<Vec<nanika_protocol::Action>, OperationError> {
        self.authorize(request.session_id)?;
        if self.navigation.busy {
            return Err(OperationError::new(Code::OperationPending));
        }
        let (actions, primary_action_id) = match &request.target {
            crate::MenuTarget::Search {
                request_id,
                result_revision,
                extension_id,
                entry_id
            } => {
                self.authorize_result(*request_id, *result_revision)?;
                if !self.navigation.stack.is_empty() {
                    return Err(OperationError::new(Code::SearchChanged));
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
                    .ok_or_else(|| OperationError::new(Code::SearchChanged))?
                    .candidate;
                let primary = candidate.actions().iter().find(|action| {
                    action.id == candidate.action_id()
                        && (action.allow_default_execution || action.confirmation_title.is_some())
                });
                (
                    candidate.actions(),
                    primary.map(|action| action.id.as_str())
                )
            }
            crate::MenuTarget::View {
                route_id,
                revision,
                item_id
            } => {
                let route = self
                    .navigation
                    .authorize_route(*route_id)
                    .map_err(|_| OperationError::new(Code::ViewChanged))?;
                if route.revision != *revision {
                    return Err(OperationError::new(Code::ViewChanged));
                }
                let actions = match (&*route.view, item_id) {
                    (nanika_protocol::View::List { list }, Some(id)) => list
                        .item(id)
                        .ok_or_else(|| OperationError::new(Code::ViewChanged))?
                        .actions
                        .as_slice(),
                    (nanika_protocol::View::Detail { detail }, None) => detail.actions.as_slice(),
                    _ => return Err(OperationError::new(Code::ActionUnavailable))
                };
                let primary = actions.iter().find(|action| {
                    action.style == nanika_protocol::ActionStyle::Primary
                        && action.allows_invocation(nanika_protocol::ActionInvocation::Default)
                });
                (actions, primary.map(|action| action.id.as_str()))
            }
        };
        nanika_protocol::validate_actions(actions)
            .map_err(|detail| OperationError::with_detail(Code::ExtensionProtocol, detail))?;
        // Keyboard and row activation own the primary action; menus expose additional actions.
        Ok(actions
            .iter()
            .filter(|action| Some(action.id.as_str()) != primary_action_id)
            .cloned()
            .collect())
    }

    pub(crate) fn request_range(
        &mut self,
        request: crate::ReadResultsRequest
    ) -> Result<(), OperationError> {
        self.authorize(request.session_id)?;
        if request.count == 0
            || request.range_id == 0
            || request.range_id > 9_007_199_254_740_991
            || request.offset.checked_add(request.count).is_none()
        {
            return Err(OperationError::new(Code::InvalidResultRange));
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
        result_revision: u64
    ) -> Result<(), OperationError> {
        if request_id != self.request_id || result_revision != self.result_revision {
            return Err(OperationError::new(Code::SearchChanged));
        }
        Ok(())
    }

    pub(crate) fn authorize(&self, session_id: u64) -> Result<(), OperationError> {
        if self.id != session_id {
            return Err(OperationError::new(Code::SessionUnavailable));
        }
        if let Some(error) = &self.transport_error {
            return Err(OperationError::with_detail(
                Code::SessionUnavailable,
                error.clone()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/SearchSession.rs"]
mod tests;
