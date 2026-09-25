use crate::{CandidateKind, normalize_query};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    kind: CandidateKind,
    entry_id: String,
    extension_id: String,
    title: String,
    subtitle: Option<String>,
    action_id: String,
    actions: Vec<nanika_protocol::Action>,
    aliases: Vec<String>,
    icon: Option<nanika_protocol::IconSource>,
    search_values: Vec<String>,
}

impl Candidate {
    pub fn new(
        kind: CandidateKind,
        extension_id: impl Into<String>,
        entry_id: impl Into<String>,
        title: impl Into<String>,
        action_id: impl Into<String>,
        actions: Vec<nanika_protocol::Action>,
        aliases: Vec<String>,
    ) -> Self {
        let title = title.into();
        let action_id = action_id.into();
        let search_values = std::iter::once(title.as_str())
            .chain(aliases.iter().map(String::as_str))
            .map(normalize_query)
            .collect();
        Self {
            kind,
            entry_id: entry_id.into(),
            extension_id: extension_id.into(),
            title,
            subtitle: None,
            actions,
            action_id,
            aliases,
            icon: None,
            search_values,
        }
    }

    pub fn kind(&self) -> CandidateKind {
        self.kind
    }

    pub fn with_icon(mut self, icon: Option<nanika_protocol::IconSource>) -> Self {
        self.icon = icon;
        self
    }

    pub fn with_subtitle(mut self, subtitle: Option<String>) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub(crate) fn search_values(&self) -> &[String] {
        &self.search_values
    }

    pub fn extension_id(&self) -> &str {
        &self.extension_id
    }

    pub fn entry_id(&self) -> &str {
        &self.entry_id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    pub fn action_id(&self) -> &str {
        &self.action_id
    }

    pub fn actions(&self) -> &[nanika_protocol::Action] {
        &self.actions
    }

    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    pub fn icon(&self) -> Option<&nanika_protocol::IconSource> {
        self.icon.as_ref()
    }

    pub(crate) fn set_extension_id(&mut self, extension_id: &str) {
        self.extension_id.clear();
        self.extension_id.push_str(extension_id);
    }
}
