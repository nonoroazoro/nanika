use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum MenuTarget {
    Search {
        request_id: u64,
        revision: u64,
        extension_id: String,
        entry_id: String,
    },
    View {
        route_id: u64,
        revision: u64,
        item_id: Option<String>,
    },
}
