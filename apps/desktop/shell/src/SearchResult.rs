use serde::Serialize;

use crate::resource_protocol;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub(crate) extension_id: String,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
    pub(crate) allow_default_execution: bool,
    pub(crate) title: String,
    pub(crate) subtitle: Option<String>,
    pub(crate) icon_url: Option<String>,
    pub(crate) contribution_icon: Option<String>,
    pub(crate) kind: String,
    pub(crate) entry_type: &'static str,
}

impl SearchResult {
    pub(crate) fn from_candidate(candidate: &nanika_search::Candidate) -> Self {
        Self {
            extension_id: candidate.extension_id().to_owned(),
            entry_id: candidate.entry_id().to_owned(),
            action_id: candidate.action_id().to_owned(),
            allow_default_execution: candidate.actions().iter().any(|action| {
                action.id == candidate.action_id()
                    && action.allows_invocation(nanika_protocol::ActionInvocation::Default)
            }),
            title: candidate.title().to_owned(),
            subtitle: candidate.subtitle().map(str::to_owned),
            icon_url: candidate.icon_key().map(|key| {
                resource_protocol::url(&format!("{}/{key}/128.png", candidate.extension_id()))
            }),
            contribution_icon: candidate.contribution_icon().map(str::to_owned),
            kind: "Extension".to_owned(),
            entry_type: match candidate.kind() {
                nanika_search::CandidateKind::Action => "action",
                nanika_search::CandidateKind::View => "view",
            },
        }
    }
}
