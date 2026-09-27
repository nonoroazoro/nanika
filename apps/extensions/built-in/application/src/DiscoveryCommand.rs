/// Work accepted by the single application discovery owner.
pub(crate) enum DiscoveryCommand {
    Refresh {
        request_id: Option<String>,
        generation: u64,
    },
    Shutdown,
}
