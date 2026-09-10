use nanika_protocol::Message;
use std::sync::{Arc, atomic::AtomicBool};

pub(crate) struct ProtocolInput {
    pub(crate) message: Message,
    pub(crate) cancelled: Arc<AtomicBool>,
}
