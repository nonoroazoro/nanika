use crate::SingleInstance;

/// The result of trying to become the host instance for the current session.
#[derive(Debug)]
pub enum InstanceRole {
    Primary(SingleInstance),
    Secondary,
}
