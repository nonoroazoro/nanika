/// Application invocation waiting for one host service response.
pub struct PendingInvocation {
    pub request_id: String,
    pub generation: u64,
}
