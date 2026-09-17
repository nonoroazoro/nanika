use crate::{CandidateKind, normalize_query};

/// One searchable entry contributed by an extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    kind: CandidateKind,
    entry_id: String,
    extension_id: String,
    title: String,
    subtitle: Option<String>,
    action_id: String,
    aliases: Vec<String>,
    icon_key: Option<String>,
    contribution_icon: Option<String>,
    search_values: Vec<String>,
}

impl Candidate {
    pub fn new(
        kind: CandidateKind,
        extension_id: impl Into<String>,
        entry_id: impl Into<String>,
        title: impl Into<String>,
        action_id: impl Into<String>,
        aliases: Vec<String>,
    ) -> Self {
        let title = title.into();
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
            action_id: action_id.into(),
            aliases,
            icon_key: None,
            contribution_icon: None,
            search_values,
        }
    }

    pub fn kind(&self) -> CandidateKind {
        self.kind
    }

    pub fn with_icon_key(mut self, icon_key: Option<String>) -> Self {
        self.icon_key = icon_key;
        self
    }

    pub fn with_contribution_icon(mut self, contribution_icon: Option<String>) -> Self {
        self.contribution_icon = contribution_icon;
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

    pub fn aliases(&self) -> &[String] {
        &self.aliases
    }

    pub fn icon_key(&self) -> Option<&str> {
        self.icon_key.as_deref()
    }

    pub fn contribution_icon(&self) -> Option<&str> {
        self.contribution_icon.as_deref()
    }

    pub(crate) fn set_extension_id(&mut self, extension_id: &str) {
        self.extension_id.clear();
        self.extension_id.push_str(extension_id);
    }
}
