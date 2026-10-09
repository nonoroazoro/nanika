use serde::Serialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum ResultIcon {
    Image { url: String },
    Symbol { name: nanika_protocol::ActionIcon }
}
