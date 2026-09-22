//! Reusable, UI-independent romanized search aliases for mixed Chinese and Latin names.

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
