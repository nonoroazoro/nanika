use crate::CandidateKind;

/// Immutable payload shared by catalog entries and ranked snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateData {
    pub(crate) _kind: CandidateKind,
    pub(crate) _entry_id: String,
    pub(crate) _extension_id: String,
    pub(crate) _title: String,
    pub(crate) _subtitle: Option<nanika_protocol::CandidateSubtitle>,
    pub(crate) _action_id: String,
    pub(crate) _actions: Vec<nanika_protocol::Action>,
    pub(crate) _aliases: Vec<String>,
    pub(crate) _icon: Option<nanika_protocol::IconSource>,
    pub(crate) _search_values: Vec<String>,
    pub(crate) _readings: Vec<nanika_text_search::RomanizedReading>,
}
