use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ResultIcon {
    Image { url: String },
    Symbol { name: nanika_protocol::ActionIcon },
}
