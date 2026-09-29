//! Shared lexical matching and optional romanized aliases, independent of catalogs and UI.

#![forbid(unsafe_code)]

#[path = "HanReadings.rs"]
mod han_readings;
mod romanization;

pub use romanization::{
    RomanizedMatch, RomanizedReading, find_romanized_match, romanized_query, romanized_readings,
};

#[cfg(test)]
#[path = "../tests/romanization.rs"]
mod romanization_tests;

mod normalization;
pub use normalization::normalize_query;
#[path = "TextQuery.rs"]
mod text_query;
pub use text_query::TextQuery;
#[path = "TextMatch.rs"]
mod text_match;
pub use text_match::TextMatch;
#[path = "TextMatcher.rs"]
mod text_matcher;
pub use text_matcher::TextMatcher;
#[cfg(test)]
#[path = "../tests/TextMatcher.rs"]
mod text_matcher_tests;
