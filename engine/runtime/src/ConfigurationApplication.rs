/// Only a live protocol acknowledgement confirms application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigurationApplication {
    Applied,
    Deferred,
}
