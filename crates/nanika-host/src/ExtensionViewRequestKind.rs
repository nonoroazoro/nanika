use nanika_protocol::ViewEvent;

/// Operation serialized onto one extension view session.
#[derive(Debug, Clone)]
pub(crate) enum ExtensionViewRequestKind {
    Event(ViewEvent),
    Close,
}
