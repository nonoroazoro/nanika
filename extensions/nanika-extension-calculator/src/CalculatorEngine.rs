use std::fmt::Write;
use std::time::{Duration, Instant};

use nanika_protocol::Candidate;
use sha2::{Digest, Sha256};

use crate::{COPY_ACTION_ID, DeadlineInterrupt};

const EVALUATION_LIMIT: Duration = Duration::from_millis(50);
const MAX_INPUT_CHARS: usize = 4_096;

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
        let query = query.trim();
        if query.is_empty() || query.chars().count() > MAX_INPUT_CHARS {
            return None;
        }
        let interrupt = DeadlineInterrupt {
            deadline: Instant::now() + EVALUATION_LIMIT,
        };
        let evaluated =
            fend_core::evaluate_preview_with_interrupt(query, &self.context, &interrupt);
        let result = evaluated.get_main_result();
        if result.is_empty() {
            return None;
        }
        Some((
            Candidate {
                entry_id: format!("calculator.{}", stable_hash(&[query, result])),
                title: format!("= {result}"),
                action_id: COPY_ACTION_ID.to_owned(),
                aliases: vec![query.to_owned()],
                icon: None,
            },
            result.to_owned(),
        ))
    }
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
