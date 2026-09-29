/// Stable identities, never labels or selectors supplied by an extension.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum SettingsSearchTarget {
    Page,
    Section { key: String },
    Field { key: String },
    Enabled,
}
