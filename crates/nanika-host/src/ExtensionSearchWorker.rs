use std::io;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use nanika_search::SearchHandle;

use crate::{
    ExtensionInvocation, ExtensionInvocationResult, ExtensionNotifier, ExtensionProcess,
    ExtensionSearchQuery, ExtensionSearchState, ExtensionWork, SupervisorError,
    publish_extension_snapshot,
};

const MAX_PENDING_INVOCATIONS: usize = 16;

/// Fixed worker that keeps extension protocol I/O off the UI thread.
pub(crate) struct ExtensionSearchWorker {
    extension_id: String,
    state: Arc<(Mutex<ExtensionSearchState>, Condvar)>,
    last_error: Arc<Mutex<Option<String>>>,
    thread: Option<JoinHandle<()>>,
}

impl ExtensionSearchWorker {
    pub(crate) fn spawn(
        extension_id: impl Into<String>,
        mut process: ExtensionProcess,
        search: SearchHandle,
        invocation_results: SyncSender<ExtensionInvocationResult>,
        notifier: ExtensionNotifier,
    ) -> io::Result<Self> {
        let extension_id = extension_id.into();
        let worker_extension_id = extension_id.clone();
        let state = Arc::new((Mutex::new(ExtensionSearchState::default()), Condvar::new()));
        let worker_state = Arc::clone(&state);
        let last_error = Arc::new(Mutex::new(None));
        let worker_error = Arc::clone(&last_error);
        let thread = std::thread::Builder::new()
            .name(format!("nanika-search-extension-{extension_id}"))
            .spawn(move || {
                if let Err(error) = process.initialize(format!("initialize-{worker_extension_id}"))
                {
                    set_error(
                        &worker_error,
                        Some(format!("extension {worker_extension_id}: {error}")),
                    );
                    notify(&notifier);
                    return;
                }
                loop {
                    let work = next_work(&worker_state);
                    let Some(work) = work else {
                        break;
                    };
                    let result = match work {
                        ExtensionWork::Query(query) => run_query(
                            &mut process,
                            &worker_extension_id,
                            query,
                            &search,
                            &worker_state,
                        ),
                        ExtensionWork::Invoke(invocation) => {
                            let result = run_invocation(
                                &mut process,
                                &worker_extension_id,
                                &invocation,
                                &worker_state,
                            );
                            let report = ExtensionInvocationResult {
                                extension_id: worker_extension_id.clone(),
                                entry_id: invocation.entry_id,
                                action_id: invocation.action_id,
                                query_context: invocation.query_context,
                                result: match &result {
                                    Ok(()) => Ok(()),
                                    Err(error) => Err(error.to_string()),
                                },
                            };
                            if invocation_results.send(report).is_err() {
                                Err(SupervisorError::ChannelClosed)
                            } else {
                                result.map(|()| true)
                            }
                        }
                    };
                    match result {
                        Ok(true) => set_error(&worker_error, None),
                        Ok(false) => {}
                        Err(error) => set_error(
                            &worker_error,
                            Some(format!("extension {worker_extension_id}: {error}")),
                        ),
                    }
                    notify(&notifier);
                }
                let _ = process.shutdown(format!("shutdown-{worker_extension_id}"));
            })?;
        Ok(Self {
            extension_id,
            state,
            last_error,
            thread: Some(thread),
        })
    }

    pub fn query(&self, generation: u64, query: impl Into<String>) {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        state.query = Some(ExtensionSearchQuery {
            generation,
            query: query.into(),
        });
        ready.notify_one();
    }

    pub(crate) fn invoke(&self, invocation: ExtensionInvocation) -> Result<(), SupervisorError> {
        let (lock, ready) = &*self.state;
        let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
        if state.invocations.len() >= MAX_PENDING_INVOCATIONS {
            return Err(SupervisorError::QueueFull);
        }
        state.invocations.push_back(invocation);
        ready.notify_one();
        Ok(())
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn last_error(&self) -> Option<String> {
        self.last_error
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    fn stop(&mut self) {
        self.request_stop();
        self.join();
    }

    pub(crate) fn request_stop(&self) {
        let (lock, ready) = &*self.state;
        lock.lock()
            .unwrap_or_else(|error| error.into_inner())
            .shutdown = true;
        ready.notify_one();
    }

    pub(crate) fn join(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for ExtensionSearchWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn next_work(state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>) -> Option<ExtensionWork> {
    let (lock, ready) = &**state;
    let mut state = lock.lock().unwrap_or_else(|error| error.into_inner());
    while state.query.is_none() && state.invocations.is_empty() && !state.shutdown {
        state = ready.wait(state).unwrap_or_else(|error| error.into_inner());
    }
    if state.shutdown {
        return None;
    }
    state
        .invocations
        .pop_front()
        .map(ExtensionWork::Invoke)
        .or_else(|| state.query.take().map(ExtensionWork::Query))
}

fn run_query(
    process: &mut ExtensionProcess,
    extension_id: &str,
    query: ExtensionSearchQuery,
    search: &SearchHandle,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
) -> Result<bool, SupervisorError> {
    recover_if_exited(process, extension_id, query.generation)?;
    let mut retried = false;
    loop {
        let result = process.query_incremental(
            format!("search-{extension_id}-{}", query.generation),
            query.generation,
            query.query.clone(),
            Duration::from_secs(2),
            |entries| {
                publish_extension_snapshot(search, extension_id, query.generation, entries)
                    .map_err(|error| SupervisorError::UnexpectedMessage(error.to_string()))
            },
            || {
                let (lock, _) = &**state;
                let state = lock.lock().unwrap_or_else(|error| error.into_inner());
                state.shutdown || state.query.is_some() || !state.invocations.is_empty()
            },
        );
        let restartable = matches!(
            &result,
            Err(SupervisorError::ChannelClosed
                | SupervisorError::Protocol(_)
                | SupervisorError::Timeout(_))
        );
        if result.is_ok() || !restartable {
            return result;
        }
        if retried {
            let _ = process.terminate();
            return result;
        }
        restart_or_terminate(
            process,
            format!("restart-{extension_id}-{}", query.generation),
        )?;
        retried = true;
    }
}

fn run_invocation(
    process: &mut ExtensionProcess,
    extension_id: &str,
    invocation: &ExtensionInvocation,
    state: &Arc<(Mutex<ExtensionSearchState>, Condvar)>,
) -> Result<(), SupervisorError> {
    recover_if_exited(process, extension_id, invocation.generation)?;
    let result = process.invoke_cancellable(
        format!("invoke-{extension_id}-{}", invocation.generation),
        invocation.generation,
        invocation.entry_id.clone(),
        invocation.action_id.clone(),
        || {
            let (lock, _) = &**state;
            lock.lock()
                .unwrap_or_else(|error| error.into_inner())
                .shutdown
        },
    );
    if matches!(
        &result,
        Err(SupervisorError::ChannelClosed
            | SupervisorError::Protocol(_)
            | SupervisorError::Timeout(_))
    ) {
        restart_or_terminate(
            process,
            format!("restart-invoke-{extension_id}-{}", invocation.generation),
        )?;
    }
    result
}

fn restart_or_terminate(
    process: &mut ExtensionProcess,
    request_id: String,
) -> Result<(), SupervisorError> {
    match process.restart(request_id) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = process.terminate();
            Err(error)
        }
    }
}

fn recover_if_exited(
    process: &mut ExtensionProcess,
    extension_id: &str,
    generation: u64,
) -> Result<(), SupervisorError> {
    process
        .recover_if_exited(format!("recover-{extension_id}-{generation}"))
        .map(|_| ())
}

fn set_error(error: &Mutex<Option<String>>, value: Option<String>) {
    *error.lock().unwrap_or_else(|error| error.into_inner()) = value;
}

fn notify(notifier: &ExtensionNotifier) {
    let notify = notifier
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    if let Some(notify) = notify {
        notify();
    }
}
