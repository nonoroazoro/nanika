use std::path::PathBuf;

use super::DiscoveryFailure;
use crate::ApplicationError;

/// Independently resolved discovery sources and their concrete failures.
#[derive(Default)]
pub(crate) struct DiscoveryRoots {
    pub paths: Vec<PathBuf>,
    pub failures: Vec<DiscoveryFailure>,
}

impl DiscoveryRoots {
    pub fn include(&mut self, source: &str, result: Result<Option<PathBuf>, ApplicationError>) {
        match result {
            Ok(Some(path)) => self.paths.push(path),
            Ok(None) => {}
            Err(error) => self.failures.push(DiscoveryFailure {
                path: None,
                message: format!("{source}: {error}"),
            }),
        }
    }
}
