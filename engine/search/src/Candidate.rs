use crate::CandidateKind;
use nanika_text_search::normalize_query;

/// A cheap immutable reference; updates replace only the affected payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    _data: std::sync::Arc<crate::CandidateData>
}

impl Candidate {
    /// Retain original presentation independently of prepared, locale-neutral search inputs.
    /// Literal titles join the index directly; references require their admitted translations.
    pub fn new(
        kind: CandidateKind,
        extension_id: impl Into<String>,
        entry_id: impl Into<String>,
        title: impl Into<nanika_protocol::LocalizedText>,
        action_id: impl Into<String>,
        actions: Vec<nanika_protocol::Action>,
        mut search_text: Vec<String>
    ) -> Self {
        let title = title.into();
        let action_id = action_id.into();
        if let nanika_protocol::LocalizedText::Literal(value) = &title
            && !search_text.contains(value)
        {
            search_text.insert(0, value.clone());
        }
        let mut search_values: Vec<String> = search_text
            .iter()
            .map(|value| normalize_query(value))
            .collect();
        let mut readings = Vec::new();
        for value in &search_text {
            for reading in nanika_text_search::romanized_readings(value) {
                for spelling in [&reading.full, &reading.initials] {
                    if !search_values.contains(spelling) {
                        search_values.push(spelling.clone());
                    }
                }
                if !readings.contains(&reading) {
                    readings.push(reading);
                }
            }
        }
        Self {
            _data: std::sync::Arc::new(crate::CandidateData {
                _kind: kind,
                _entry_id: entry_id.into(),
                _extension_id: extension_id.into(),
                _title: title,
                _subtitle: None,
                _actions: actions,
                _action_id: action_id,
                _icon: None,
                _search_values: search_values,
                _readings: readings
            })
        }
    }

    pub(crate) fn readings(&self) -> &[nanika_text_search::RomanizedReading] {
        &self._data._readings
    }

    pub fn kind(&self) -> CandidateKind {
        self._data._kind
    }

    pub fn with_icon(mut self, icon: Option<nanika_protocol::IconSource>) -> Self {
        std::sync::Arc::make_mut(&mut self._data)._icon = icon;
        self
    }

    pub fn with_subtitle(mut self, subtitle: Option<nanika_protocol::CandidateSubtitle>) -> Self {
        std::sync::Arc::make_mut(&mut self._data)._subtitle = subtitle;
        self
    }

    pub(crate) fn search_values(&self) -> &[String] {
        &self._data._search_values
    }

    pub fn extension_id(&self) -> &str {
        &self._data._extension_id
    }

    pub fn entry_id(&self) -> &str {
        &self._data._entry_id
    }

    pub fn title(&self) -> &nanika_protocol::LocalizedText {
        &self._data._title
    }

    pub fn subtitle(&self) -> Option<&nanika_protocol::CandidateSubtitle> {
        self._data._subtitle.as_ref()
    }

    pub fn action_id(&self) -> &str {
        &self._data._action_id
    }

    pub fn actions(&self) -> &[nanika_protocol::Action] {
        &self._data._actions
    }

    pub fn icon(&self) -> Option<&nanika_protocol::IconSource> {
        self._data._icon.as_ref()
    }

    /// Compares the reviewed target. Search aliases and icons cannot retarget an action.
    pub(crate) fn same_execution_target(&self, other: &Self) -> bool {
        if std::sync::Arc::ptr_eq(&self._data, &other._data) {
            return true;
        }
        self.kind() == other.kind()
            && self.extension_id() == other.extension_id()
            && self.entry_id() == other.entry_id()
            && self.title() == other.title()
            && self.subtitle() == other.subtitle()
            && self.action_id() == other.action_id()
            && self.actions().len() == other.actions().len()
            && self.actions().iter().zip(other.actions()).all(|(a, b)| {
                a.id == b.id
                    && a.title == b.title
                    && a.confirmation_title == b.confirmation_title
                    && a.confirmation_description == b.confirmation_description
                    && a.style == b.style
                    && a.enabled == b.enabled
                    && a.allow_default_execution == b.allow_default_execution
                    && a.group == b.group
            })
    }

    pub(crate) fn set_extension_id(&mut self, extension_id: &str) {
        if self.extension_id() != extension_id {
            let data = std::sync::Arc::make_mut(&mut self._data);
            data._extension_id.clear();
            data._extension_id.push_str(extension_id);
        }
    }
}
