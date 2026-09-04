use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub(crate) extension_id: String,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) icon_url: Option<String>,
    pub(crate) kind: String,
}

impl SearchResult {
    pub(crate) fn from_candidate(candidate: &nanika_search::Candidate) -> Self {
        Self {
            extension_id: candidate.extension_id().to_owned(),
            entry_id: candidate.entry_id().to_owned(),
            action_id: candidate.action_id().to_owned(),
            title: candidate.title().to_owned(),
            subtitle: candidate.subtitle().map(str::to_owned),
            icon_url: candidate.icon_key().map(|key| {
                format!(
                    "nanika-icon://localhost/{}/{key}/128.png",
                    candidate.extension_id()
                )
            }),
            kind: "Extension".to_owned(),
        }
    }
}
