use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::{DesktopRuntime, RootSearchSnapshot, SearchPhase};

const VISIBLE_ENTRY_PREPARATION_LIMIT: usize = 10;

pub(crate) enum SearchDelivery {
    Wake,
    Shutdown,
}

/// The sole Channel writer. Core callbacks only wake it, so transport never blocks
/// a search or extension owner. The next update waits for the WebView acknowledgement.
pub(crate) fn run_delivery(
    shared: &Mutex<DesktopRuntime>,
    wakes: Receiver<SearchDelivery>,
    settings: &Mutex<crate::SettingsApplications>,
) {
    run_delivery_with_preparation(shared, wakes, settings, |runtime, snapshot| {
        runtime.prepare_visible_entries(snapshot, VISIBLE_ENTRY_PREPARATION_LIMIT);
    });
}

pub(crate) fn run_delivery_with_preparation(
    shared: &Mutex<DesktopRuntime>,
    wakes: Receiver<SearchDelivery>,
    settings: &Mutex<crate::SettingsApplications>,
    prepare: impl Fn(&nanika_host::RuntimeService, &nanika_search::SearchSnapshot),
) {
    while let Ok(event) = wakes.recv() {
        if matches!(event, SearchDelivery::Shutdown) {
            break;
        }
        let runtime = shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .runtime
            .clone();
        let lifecycle = runtime.as_ref().and_then(|runtime| {
            let mut settings = settings.lock().unwrap_or_else(|error| error.into_inner());
            settings.refresh_lifecycle(runtime);
            settings.next_lifecycle()
        });
        if let Some((channel, event)) = lifecycle
            && let Err(error) = channel.send(event)
        {
            settings
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .disconnect(channel.id());
            tracing::error!(%error, "settings lifecycle delivery failed");
        }
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
        let active = state
            .runtime
            .as_ref()
            .map(|runtime| runtime.extension_info());
        if let (Some(infos), Some(session)) = (active, state.session.as_mut()) {
            let previous = session.navigation.stack.len();
            session.navigation.stack.retain(|route| {
                infos.iter().any(|info| {
                    info.id == route.extension_id
                        && info.enabled
                        && info.instance_id == Some(route.instance_id)
                })
            });
            // Other extensions still own their views. Removing a lower route must
            // not silently discard an active extension's resources above it.
            if session.navigation.stack.len() != previous {
                session.navigation.revision += 1;
            }
        }
        let DesktopRuntime {
            runtime,
            session,
            startup_error,
        } = &mut *state;
        let Some(session) = session else {
            continue;
        };
        if session.transport_error.is_some() {
            continue;
        }
        let latest = runtime
            .as_ref()
            .and_then(|runtime| runtime.latest_snapshot())
            .filter(|snapshot| snapshot.generation == session.generation);
        if session.in_flight.is_some() {
            continue;
        }
        let active_error = runtime.as_ref().and_then(|runtime| runtime.active_error());
        let phase = if startup_error.is_some() || active_error.is_some() {
            SearchPhase::Error
        } else if latest.is_some() {
            SearchPhase::Ready
        } else {
            SearchPhase::Searching
        };
        let error = if startup_error.is_some() {
            Some("Nanika could not start. Open diagnostics for details.".to_owned())
        } else {
            active_error
        };
        let warnings = runtime
            .as_ref()
            .map_or_else(Vec::new, |runtime| runtime.search_warnings());
        let unchanged = session.phase == Some(phase)
            && session.error == error
            && session.warnings == warnings
            && match (&latest, &session.delivered) {
                (Some(next), Some(previous)) => Arc::ptr_eq(next, previous),
                (None, None) => true,
                _ => false,
            };
        if unchanged && session.delivered_navigation_revision == session.navigation.revision {
            continue;
        }
        let results_changed = session.phase.is_none()
            || match (&latest, &session.delivered) {
                (Some(next), Some(previous)) => !Arc::ptr_eq(next, previous),
                (None, None) => false,
                _ => true,
            };
        let route = session
            .navigation
            .stack
            .last()
            .map(|route| (route.route_id, route.revision));
        let mut navigation = session.navigation.snapshot();
        if session.revision != 0 && route == session.delivered_route {
            navigation.current = None;
        }
        session.revision += 1;
        let mut update = RootSearchSnapshot {
            navigation,
            session_id: session.id,
            request_id: session.request_id,
            revision: session.revision,
            query: session.query.clone(),
            results: None,
            phase,
            error: error.clone(),
            warnings: warnings.clone(),
        };
        let updates = session.updates.clone();
        let runtime = runtime.clone();
        // Reserve before unlocking: a synchronous callback can acknowledge during
        // send. No result conversion, view serialization or IPC owns shared state.
        session.in_flight = Some((session.revision, Instant::now()));
        session.phase = Some(phase);
        session.error = error;
        session.warnings = warnings;
        session.delivered = latest.clone();
        session.delivered_navigation_revision = session.navigation.revision;
        session.delivered_route = route;
        let session_id = session.id;
        let revision = session.revision;
        drop(state);
        // Preparation completion wakes delivery too. Only a changed search
        // snapshot schedules preparation, never navigation or acknowledgements.
        if results_changed && let (Some(runtime), Some(snapshot)) = (&runtime, &latest) {
            prepare(runtime, snapshot);
        }
        if results_changed {
            update.results = Some(latest.as_ref().map_or_else(Vec::new, |snapshot| {
                snapshot
                    .results
                    .iter()
                    .map(|ranked| crate::SearchResult::from_candidate(&ranked.candidate))
                    .collect()
            }));
        }
        let count = update.results.as_ref().map(Vec::len);
        match updates.send(update) {
            Ok(()) => {
                tracing::debug!(
                    session_id,
                    revision,
                    ?phase,
                    results = ?count,
                    "search update queued for WebView"
                );
            }
            Err(error) => {
                tracing::error!(session_id, revision, %error, "search Channel send failed");
                let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
                if let Some(session) = state
                    .session
                    .as_mut()
                    .filter(|session| session.id == session_id)
                {
                    session.transport_error =
                        Some("Search updates could not be sent. Reload the window.".to_owned());
                }
            }
        }
    }
}
