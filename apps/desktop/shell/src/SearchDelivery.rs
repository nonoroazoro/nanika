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
pub(crate) fn run_delivery(shared: &Mutex<DesktopRuntime>, wakes: Receiver<SearchDelivery>) {
    while let Ok(event) = wakes.recv() {
        if matches!(event, SearchDelivery::Shutdown) {
            break;
        }
        let mut state = shared.lock().unwrap_or_else(|error| error.into_inner());
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
        if let (Some(runtime), Some(snapshot)) = (runtime.as_ref(), latest.as_deref()) {
            runtime.prepare_visible_entries(snapshot, VISIBLE_ENTRY_PREPARATION_LIMIT);
        }
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
        session.revision += 1;
        let update = RootSearchSnapshot {
            navigation: session.navigation.snapshot(),
            session_id: session.id,
            request_id: session.request_id,
            revision: session.revision,
            query: session.query.clone(),
            results: latest.as_ref().map_or_else(Vec::new, |snapshot| {
                snapshot
                    .results
                    .iter()
                    .map(|ranked| crate::SearchResult::from_candidate(&ranked.candidate))
                    .collect()
            }),
            phase,
            error: error.clone(),
            warnings: warnings.clone(),
        };
        let count = update.results.len();
        match session.updates.send(update) {
            Ok(()) => {
                session.in_flight = Some((session.revision, Instant::now()));
                session.phase = Some(phase);
                session.error = error;
                session.warnings = warnings;
                session.delivered = latest;
                session.delivered_navigation_revision = session.navigation.revision;
                tracing::debug!(
                    session_id = session.id,
                    request_id = session.request_id,
                    revision = session.revision,
                    ?phase,
                    results = count,
                    "search update queued for WebView"
                );
            }
            Err(error) => {
                tracing::error!(session_id = session.id, request_id = session.request_id,
                    revision = session.revision, %error, "search Channel send failed");
                session.transport_error =
                    Some("Search updates could not be sent. Reload the window.".to_owned());
            }
        }
    }
}
