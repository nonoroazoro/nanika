use crate::QueryInterrupt;
use std::fmt::Write;
use std::sync::atomic::AtomicBool;

use nanika_protocol::{Candidate, CandidateKind};
use sha2::{Digest, Sha256};

use crate::COPY_ACTION_ID;

/// Deterministic calculator preview context reused by the extension process.
pub struct CalculatorEngine {
    context: fend_core::Context,
}

impl CalculatorEngine {
    pub fn new() -> Self {
        Self {
            context: fend_core::Context::new(),
        }
    }

    pub fn evaluate(&self, query: &str) -> Option<(Candidate, String)> {
        self.evaluate_cancellable(query, &AtomicBool::new(false))
    }

    pub fn evaluate_cancellable(
        &self,
        query: &str,
        cancelled: &AtomicBool,
    ) -> Option<(Candidate, String)> {
        let query = query.trim();
        if query.is_empty() || !has_explicit_operator(query) {
            return None;
        }
        let interrupt = QueryInterrupt::new(cancelled);
        let evaluated =
            fend_core::evaluate_preview_with_interrupt(query, &self.context, &interrupt);
        let result = evaluated.get_main_result();
        if result.is_empty() {
            return None;
        }
        Some((
            Candidate {
                kind: CandidateKind::Action,
                entry_id: format!("calculator.{}", stable_hash(&[query, result])),
                title: format!("= {result}"),
                subtitle: Some("Calculator".to_owned()),
                action_id: COPY_ACTION_ID.to_owned(),
                actions: vec![nanika_protocol::Action::primary(
                    COPY_ACTION_ID,
                    "Copy result",
                )],
                aliases: vec![query.to_owned()],
                icon: None,
                contribution_icon: None,
            },
            result.to_owned(),
        ))
    }
}

fn has_explicit_operator(query: &str) -> bool {
    query.chars().any(|character| {
        matches!(
            character,
            '+' | '-'
                | '*'
                | '/'
                | '^'
                | '%'
                | '='
                | '<'
                | '>'
                | '&'
                | '|'
                | '!'
                | '('
                | ')'
                | '×'
                | '÷'
                | '−'
        )
    }) || query.split_whitespace().any(|token| {
        matches!(
            token.to_ascii_lowercase().as_str(),
            "to" | "in" | "of" | "mod" | "per" | "plus" | "minus" | "times" | "divided"
        )
    })
}

impl Default for CalculatorEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn stable_hash(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update(part.as_bytes());
        digest.update([0]);
    }
    let digest = digest.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}
