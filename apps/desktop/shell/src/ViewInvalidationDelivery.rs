use nanika_host::{OperationError, OperationErrorCode as Code};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use crate::{DesktopRuntime, SearchDelivery};

pub(crate) enum ViewInvalidationDelivery {
    Wake,
    Shutdown
}

/// Refreshes extension-owned views independently from Root Search delivery.
/// Extension completion may block, so this worker must never own the search Channel.
pub(crate) fn run_delivery(
    shared: &Mutex<DesktopRuntime>,
    wakes: Receiver<ViewInvalidationDelivery>,
    search_wakes: &SyncSender<SearchDelivery>
) {
    while let Ok(event) = wakes.recv() {
        if matches!(event, ViewInvalidationDelivery::Shutdown) {
            break;
        }
        refresh_invalidated_views(shared, search_wakes);
        if matches!(
            search_wakes.try_send(SearchDelivery::Wake),
            Err(TrySendError::Disconnected(_))
        ) {
            break;
        }
    }
}

fn refresh_invalidated_views(
    shared: &Mutex<DesktopRuntime>,
    search_wakes: &SyncSender<SearchDelivery>
) {
    let invalidations = {
        let state = shared.lock().unwrap_or_else(|error| error.into_inner());
        state
            .runtime
            .as_ref()
            .map_or_else(Vec::new, |runtime| runtime.take_view_invalidations())
    };
    // Revoke cached resources before waiting for a potentially slow view refresh.
    {
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(session) = state.session.as_mut() {
            for invalidation in &invalidations {
                if !invalidation.text.is_empty() {
                    session.navigation.revoke_text(
                        &invalidation.extension_id,
                        invalidation.instance_id,
                        &invalidation.text
                    );
                    let notice = session
                        .text_invalidations
                        .entry((invalidation.extension_id.clone(), invalidation.instance_id))
                        .or_insert_with(|| crate::TextInvalidationNotice {
                            extension_id: invalidation.extension_id.clone(),
                            instance_id: invalidation.instance_id,
                            text: Default::default()
                        });
                    notice.text.merge(invalidation.text.clone());
                }
            }
        }
    }
    let _ = search_wakes.try_send(SearchDelivery::Wake);
    for invalidation in invalidations {
        let Some((session_id, operation_lock)) = ({
            let state = shared.lock().unwrap_or_else(|error| error.into_inner());
            state
                .session
                .as_ref()
                .map(|session| (session.id, Arc::clone(&session.view_operation_lock)))
        }) else {
            continue;
        };
        let _operation_guard = operation_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some((runtime, route)) = ({
            let state = shared.lock().unwrap_or_else(|error| error.into_inner());
            state.runtime.as_ref().and_then(|runtime| {
                state
                    .session
                    .as_ref()
                    .filter(|session| session.id == session_id)
                    .and_then(|session| session.navigation.stack.last())
                    .filter(|route| {
                        route.instance_id == invalidation.instance_id
                            && route.extension_id == invalidation.extension_id
                    })
                    .cloned()
                    .map(|route| (Arc::clone(runtime), route))
            })
        }) else {
            continue;
        };
        let result = _refresh_route(shared, &runtime, &route);
        let completion = match result {
            Ok(completion) => completion,
            Err(error) => {
                tracing::warn!(extension_id = invalidation.extension_id, view_id = route.view_id,
                    %error, "extension view invalidation failed");
                let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
                runtime.with_extension_instance(&route.extension_id, route.instance_id, || {
                    apply_failure(&mut state, session_id, &route, error);
                });
                continue;
            }
        };
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        runtime.with_extension_instance(&route.extension_id, route.instance_id, || {
            apply_completion(
                &mut state,
                session_id,
                &route,
                completion.revision,
                completion.view
            );
        });
    }
}

pub(crate) fn apply_completion(
    state: &mut DesktopRuntime,
    session_id: u64,
    route: &crate::ExtensionViewSnapshot,
    revision: u64,
    view: Option<nanika_protocol::View>
) {
    let Some(session) = _current_session(state, session_id, route) else {
        return;
    };
    session
        .navigation
        .finish_refresh(route.route_id, Ok(view.map(|view| (revision, view))));
}

pub(crate) fn apply_failure(
    state: &mut DesktopRuntime,
    session_id: u64,
    route: &crate::ExtensionViewSnapshot,
    error: OperationError
) {
    if let Some(session) = _current_session(state, session_id, route) {
        session
            .navigation
            .finish_refresh(route.route_id, Err(error));
    }
}

fn _refresh_route(
    shared: &Mutex<DesktopRuntime>,
    runtime: &nanika_host::RuntimeService,
    route: &crate::ExtensionViewSnapshot
) -> Result<nanika_host::RuntimeViewCompletion, OperationError> {
    let result = runtime
        .view_event(
            &route.extension_id,
            route.instance_id,
            route.generation,
            &route.view_id,
            route.revision,
            nanika_protocol::ViewEvent::Invalidated
        )
        .and_then(|completion| {
            completion.recv().map_err(|_| {
                OperationError::with_detail(
                    Code::ViewOperationUnconfirmed,
                    "Extension closed without an invalidated view result."
                )
            })?
        });
    result.and_then(|completion| {
        let already_owned = if let nanika_protocol::NavigationEffect::Push { view_id, .. } =
            &completion.effect
        {
            shared
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .session
                .as_ref()
                .is_some_and(|session| {
                    session.navigation.stack.iter().any(|current| {
                        current.extension_id == route.extension_id && current.view_id == *view_id
                    })
                })
        } else {
            false
        };
        _accept_refresh_effect(&completion.effect, already_owned, |view_id, revision| {
            crate::close_runtime_view(
                runtime,
                &route.extension_id,
                route.instance_id,
                route.generation,
                view_id,
                revision
            )
        })?;
        Ok(completion)
    })
}

fn _current_session<'a>(
    state: &'a mut DesktopRuntime,
    session_id: u64,
    route: &crate::ExtensionViewSnapshot
) -> Option<&'a mut crate::SearchSession> {
    state.session.as_mut().filter(|session| {
        session.id == session_id
            && session.navigation.stack.last().is_some_and(|current| {
                current.instance_id == route.instance_id
                    && current.route_id == route.route_id
                    && current.revision == route.revision
                    && current.extension_id == route.extension_id
                    && current.view_id == route.view_id
            })
    })
}

fn _accept_refresh_effect(
    effect: &nanika_protocol::NavigationEffect,
    already_owned: bool,
    close: impl FnOnce(&str, u64) -> Result<(), OperationError>
) -> Result<(), OperationError> {
    if *effect == nanika_protocol::NavigationEffect::None {
        return Ok(());
    }
    let primary = OperationError::with_detail(
        Code::ExtensionProtocol,
        "A background view refresh returned a navigation effect."
    );
    // A rejected publication must not leak its new view or close an existing route.
    if !already_owned
        && let nanika_protocol::NavigationEffect::Push {
            view_id, revision, ..
        } = effect
    {
        close(view_id, *revision).map_err(|cleanup| OperationError::Cleanup {
            primary: Box::new(primary.clone()),
            cleanup: Box::new(cleanup)
        })?;
    }
    Err(primary)
}

#[cfg(test)]
#[path = "../tests/ViewInvalidationDelivery.rs"]
mod tests;
