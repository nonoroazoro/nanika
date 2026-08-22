//! Built-in explicit shell command extension.

use std::fmt::Write;

use nanika_protocol::Candidate;
use sha2::{Digest, Sha256};

pub const EXTENSION_ID: &str = "com.nanika.command";
pub const RUN_ACTION_ID: &str = "command.run";

pub fn command_candidate(query: &str) -> Option<(Candidate, String)> {
    let command = query.trim().strip_prefix('>')?.trim();
    if command.is_empty() {
        return None;
    }
    let entry_id = format!("command.{}", stable_hash(command));
    Some((
        Candidate {
            entry_id,
            title: format!("Run command: {command}"),
            action_id: RUN_ACTION_ID.to_owned(),
            aliases: vec![query.to_owned()],
        },
        command.to_owned(),
    ))
}

fn stable_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}
