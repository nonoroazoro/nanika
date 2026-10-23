use nanika_host::HostServiceHandler;
use nanika_protocol::{HostServiceRequest, HostServiceResponse};
use std::sync::{Mutex, mpsc};

#[derive(Default)]
pub struct PendingHostService {
    _preparation_error: Option<String>,
    _admission_error: Option<String>,
    _on_prepare: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
    _preparations: std::sync::atomic::AtomicUsize,
    _admissions: std::sync::atomic::AtomicUsize,
    response: Mutex<Option<mpsc::SyncSender<Result<HostServiceResponse, String>>>>
}
impl PendingHostService {
    pub fn stop_during_prepare(stop: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            _on_prepare: Some(std::sync::Arc::new(stop)),
            ..Default::default()
        }
    }
    pub fn reject_preparation(detail: &str) -> Self {
        Self {
            _preparation_error: Some(detail.into()),
            ..Default::default()
        }
    }
    pub fn reject_admission(detail: &str) -> Self {
        Self {
            _admission_error: Some(detail.into()),
            ..Default::default()
        }
    }
    pub fn prepared(&self) -> bool {
        self._preparations.load(std::sync::atomic::Ordering::SeqCst) != 0
    }
    pub fn admissions(&self) -> usize {
        self._admissions.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn submitted(&self) -> bool {
        self.response.lock().unwrap().is_some()
    }
    pub fn disconnect(&self) {
        self.response
            .lock()
            .unwrap()
            .take()
            .expect("accepted receipt");
    }
    pub fn complete(&self, result: Result<HostServiceResponse, String>) -> bool {
        self.response
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(result)
            .is_ok()
    }
}
impl HostServiceHandler for PendingHostService {
    fn prepare(
        &self,
        _: &str,
        _: HostServiceRequest,
        _: &mut dyn FnMut() -> nanika_host::ExtensionInterruption
    ) -> Result<nanika_host::PreparedHostService<'_>, String> {
        self._preparations
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(stop) = &self._on_prepare {
            stop();
        }
        if let Some(error) = &self._preparation_error {
            return Err(error.clone());
        }
        Ok(nanika_host::PreparedHostService::new(move || {
            if let Some(error) = &self._admission_error {
                return Err(error.clone());
            }
            self._admissions
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let (response, receiver) = mpsc::sync_channel(1);
            *self.response.lock().unwrap() = Some(response);
            Ok(receiver)
        }))
    }
}
