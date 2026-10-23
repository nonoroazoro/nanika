use serde::Serialize;

use crate::resource_protocol;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub(crate) instance_id: u64,
    pub(crate) resource_scope: Option<nanika_host::ExtensionResourceScope>,
    pub(crate) extension_id: String,
    pub(crate) entry_id: String,
    pub(crate) action_id: String,
    pub(crate) action_title: nanika_protocol::LocalizedText,
    pub(crate) action_style: nanika_protocol::ActionStyle,
    pub(crate) allow_default_execution: bool,
    pub(crate) confirmation_title: Option<nanika_protocol::LocalizedText>,
    pub(crate) confirmation_description: Option<nanika_protocol::LocalizedText>,
    pub(crate) title: nanika_protocol::LocalizedText,
    pub(crate) subtitle: Option<nanika_protocol::CandidateSubtitle>,
    pub(crate) icon: Option<crate::result_icon::ResultIcon>,
    pub(crate) kind: String,
    pub(crate) entry_type: nanika_search::CandidateKind
}

impl SearchResult {
    pub(crate) fn from_candidate(
        candidate: &nanika_search::Candidate,
        extension_icon: Option<nanika_protocol::IconSource>
    ) -> Self {
        let primary = candidate
            .actions()
            .iter()
            .find(|action| action.id == candidate.action_id());
        let image = |source: &nanika_protocol::IconSource| {
            resource_protocol::icon_url(candidate.extension_id(), source)
                .map(|url| crate::result_icon::ResultIcon::Image { url })
        };
        let icon = match candidate.icon() {
            Some(source) => image(source),
            None => primary
                .and_then(|action| action.icon)
                .map(|name| crate::result_icon::ResultIcon::Symbol { name })
                .or_else(|| extension_icon.as_ref().and_then(image))
        };
        Self {
            instance_id: 0,
            resource_scope: None,
            extension_id: candidate.extension_id().to_owned(),
            entry_id: candidate.entry_id().to_owned(),
            action_id: candidate.action_id().to_owned(),
            action_title: primary
                .map(|action| action.title.clone())
                .unwrap_or_else(|| candidate.title().clone()),
            action_style: primary
                .map_or(nanika_protocol::ActionStyle::Primary, |action| action.style),
            allow_default_execution: candidate.actions().iter().any(|action| {
                action.id == candidate.action_id()
                    && action.allows_invocation(nanika_protocol::ActionInvocation::Default)
            }),
            confirmation_title: candidate
                .actions()
                .iter()
                .find(|action| action.id == candidate.action_id() && action.enabled)
                .and_then(|action| action.confirmation_title.clone()),
            confirmation_description: primary
                .filter(|action| action.enabled)
                .and_then(|action| action.confirmation_description.clone()),
            title: candidate.title().to_owned(),
            subtitle: candidate.subtitle().cloned(),
            icon,
            kind: "Extension".to_owned(),
            entry_type: candidate.kind()
        }
    }
}

#[cfg(test)]
#[path = "../tests/SearchResult.rs"]
mod tests;
