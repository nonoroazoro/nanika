use crate::{PreparedLocale, SupportedLocale};
use serde::Serialize;
use std::collections::BTreeMap;

/// Content identity is computed by Rust. The bridge must additionally bind access to its owner/session.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedResources {
    pub identity: String,
    pub default_locale: SupportedLocale,
    pub locales: BTreeMap<SupportedLocale, PreparedLocale>
}
