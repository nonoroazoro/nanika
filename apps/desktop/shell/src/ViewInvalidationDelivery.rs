use std::sync::mpsc::{Receiver, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

use crate::{DesktopRuntime, SearchDelivery};

pub(crate) enum ViewInvalidationDelivery {
    Wake,
    Shutdown,
}

/// Refreshes extension-owned views independently from Root Search delivery.
/// Extension completion may block, so this worker must never own the search Channel.
pub(crate) fn run_delivery(
    shared: &Mutex<DesktopRuntime>,
    view_operation_lock: &Mutex<()>,
    wakes: Receiver<ViewInvalidationDelivery>,
    search_wakes: &SyncSender<SearchDelivery>,
) {
    while let Ok(event) = wakes.recv() {
        if matches!(event, ViewInvalidationDelivery::Shutdown) {
            break;
        }
        refresh_invalidated_views(shared, view_operation_lock);
        if matches!(
            search_wakes.try_send(SearchDelivery::Wake),
            Err(TrySendError::Disconnected(_))
        ) {
            break;
        }
    }
}

fn refresh_invalidated_views(shared: &Mutex<DesktopRuntime>, view_operation_lock: &Mutex<()>) {
    let invalidations = {
        let state = shared.lock().unwrap_or_else(|error| error.into_inner());
        state
            .runtime
            .as_ref()
            .map_or_else(Vec::new, |runtime| runtime.take_view_invalidations())
    };
    for invalidation in invalidations {
        let _operation_guard = view_operation_lock
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let Some((runtime, route)) = ({
            let state = shared.lock().unwrap_or_else(|error| error.into_inner());
            state.runtime.as_ref().and_then(|runtime| {
                state
                    .session
                    .as_ref()
                    .and_then(|session| session.navigation.stack.last())
                    .filter(|route| {
                        route.extension_id == invalidation.extension_id
                            && route.view_id == invalidation.view_id
                    })
                    .cloned()
                    .map(|route| (Arc::clone(runtime), route))
            })
        }) else {
            continue;
        };
        let result = runtime
            .view_event(
                &route.extension_id,
                route.generation,
                &route.view_id,
                route.revision,
                nanika_protocol::ViewEvent::Invalidated,
            )
            .and_then(|completion| {
                completion.recv().map_err(|_| {
                    "Extension closed without an invalidated view result.".to_owned()
                })?
            });
        let Ok(completion) = result else {
            tracing::warn!(
                extension_id = invalidation.extension_id,
                view_id = invalidation.view_id,
                error = %result.expect_err("failed invalidation"),
                "extension view invalidation failed"
            );
            continue;
        };
        if completion.effect != nanika_protocol::NavigationEffect::None {
            tracing::warn!(
                extension_id = invalidation.extension_id,
                view_id = invalidation.view_id,
                "extension view invalidation returned a navigation effect"
            );
            continue;
        }
        let Some(view) = completion.view else {
            continue;
        };
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        let Some(current) = state
            .session
            .as_mut()
            .and_then(|session| session.navigation.stack.last_mut())
            .filter(|current| {
                current.route_id == route.route_id && current.revision == route.revision
            })
        else {
            continue;
        };
        current.view = view;
        current.revision = completion.revision;
        if let Some(session) = state.session.as_mut() {
            session.navigation.revision = session.navigation.revision.saturating_add(1);
        }
    }
}
