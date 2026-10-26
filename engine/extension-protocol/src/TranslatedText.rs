use icu_locale::{LanguageIdentifier, Locale, LocaleCanonicalizer, LocaleExpander};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Translations supplied by a domain source, without a package resource lookup.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TranslatedText {
    pub fallback: String,
    pub translations: BTreeMap<String, String>
}

impl TranslatedText {
    pub fn validate(&self) -> Result<(), String> {
        if self.translations.is_empty() {
            return Err("translated text requires at least one locale".into());
        }
        let mut bytes = self.fallback.len();
        for (locale, text) in &self.translations {
            if locale.len() > 64 || canonical_language_tag(locale).as_ref() != Some(locale) {
                return Err("translated text locale must be a canonical language tag".into());
            }
            bytes = bytes
                .saturating_add(locale.len())
                .saturating_add(text.len());
        }
        if bytes > 65_536 {
            return Err("translated text exceeds 65536 bytes".into());
        }
        Ok(())
    }

    /// UI locale never removes other translations from the searchable source data.
    pub fn resolve(&self, locale: &str) -> &str {
        self._match(locale)
            .or_else(|| self._match("en"))
            .unwrap_or(&self.fallback)
    }

    pub fn values(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.fallback.as_str())
            .chain(self.translations.values().map(String::as_str))
    }

    fn _match(&self, locale: &str) -> Option<&str> {
        if let Some(value) = self.translations.get(locale) {
            return Some(value);
        }
        let expander = LocaleExpander::new_extended();
        let mut requested = _language(locale)?;
        let language = requested.language;
        expander.maximize(&mut requested);
        let mut best = None;
        for (locale, value) in &self.translations {
            let Some(mut available) = _language(locale) else {
                continue;
            };
            // Canonical source tags establish language identity; likely subtags only rank variants.
            if available.language != language {
                continue;
            }
            expander.maximize(&mut available);
            let score = 2 * u8::from(available.script == requested.script)
                + u8::from(available.region == requested.region);
            // Canonical tags are sorted; ties retain the first tag for deterministic Rust/JS output.
            if best.is_none_or(|(previous, _)| score > previous) {
                best = Some((score, value.as_str()));
            }
        }
        best.map(|(_, value)| value)
    }
}

/// Normalize source language aliases before publishing canonical protocol keys.
pub fn canonical_language_tag(value: &str) -> Option<String> {
    _language(value).map(|value| value.to_string())
}

fn _language(value: &str) -> Option<LanguageIdentifier> {
    let mut locale: Locale = value.parse::<LanguageIdentifier>().ok()?.into();
    LocaleCanonicalizer::new_extended().canonicalize(&mut locale);
    Some(locale.id)
}
