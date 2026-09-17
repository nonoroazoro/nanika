/// Activation semantics for one searchable entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Action,
    View,
}
