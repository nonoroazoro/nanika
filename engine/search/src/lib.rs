//! Search aggregation, deterministic ranking, and input history.

#![forbid(unsafe_code)]

#[path = "Candidate.rs"]
mod candidate;
mod constants;
#[path = "InputHistory.rs"]
mod input_history;
#[path = "MatchContext.rs"]
mod match_context;
mod normalization;
#[path = "RankedCandidate.rs"]
mod ranked_candidate;
mod ranking;
#[path = "SearchCommand.rs"]
mod search_command;
#[path = "SearchEngine.rs"]
mod search_engine;
#[path = "SearchHandle.rs"]
mod search_handle;
#[path = "SearchNotifier.rs"]
mod search_notifier;
#[path = "SearchOwner.rs"]
mod search_owner;
#[path = "SearchQueueError.rs"]
mod search_queue_error;
#[path = "SearchSnapshot.rs"]
mod search_snapshot;
#[path = "UsageKey.rs"]
mod usage_key;
#[path = "UsageMap.rs"]
mod usage_map;
#[path = "UsageStat.rs"]
mod usage_stat;

pub use candidate::*;
pub use input_history::*;
pub(crate) use match_context::*;
pub use ranked_candidate::*;
pub(crate) use search_command::*;
pub use search_engine::*;
pub use search_handle::*;
pub(crate) use search_notifier::*;
pub use search_owner::*;
pub use search_queue_error::*;
pub use search_snapshot::*;
pub use usage_key::*;
pub use usage_map::*;
pub use usage_stat::*;

pub use constants::{MAX_QUERY_CHARS, MAX_USAGE_ROWS, USAGE_RETENTION_DAYS};
pub use normalization::{normalize_history_key, normalize_query};
