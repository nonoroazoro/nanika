use std::sync::{Mutex, atomic::AtomicBool};

#[derive(Default)]
pub(crate) struct SettingsWindow {
    pub(crate) creation: Mutex<()>,
    pub(crate) directory_picker: Mutex<()>,
    pub(crate) ready: AtomicBool,
    pub(crate) requested: AtomicBool,
    pub(crate) maximized: AtomicBool,
}
