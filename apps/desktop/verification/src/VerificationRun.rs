use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

pub struct VerificationRun {
    pub output: PathBuf,
    pub completed: AtomicBool
}
