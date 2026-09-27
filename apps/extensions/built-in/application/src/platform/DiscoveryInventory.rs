use crate::{ApplicationEntry, ApplicationError};

/// An authoritative native source, separate from traversable filesystem roots.
pub(crate) struct DiscoveryInventory {
    pub key: &'static str,
    pub entries: Result<Vec<ApplicationEntry>, ApplicationError>,
}
