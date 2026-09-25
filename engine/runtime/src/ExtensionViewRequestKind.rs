use nanika_protocol::ViewEvent;

#[derive(Debug, Clone)]
pub(crate) enum ExtensionViewRequestKind {
    Event(ViewEvent),
    Close,
}
