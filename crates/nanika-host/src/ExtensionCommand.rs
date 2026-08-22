use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub(crate) struct ExtensionCommand {
    pub(crate) program: PathBuf,
    pub(crate) arguments: Vec<OsString>,
}
