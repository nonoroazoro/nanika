use nanika_protocol::ExtensionConfiguration;

const MAX_ENTRIES_KEY: &str = "clipboard.maxEntries";
const MAX_AGE_DAYS_KEY: &str = "clipboard.maxAgeDays";
const MIN_MAX_ENTRIES: u32 = 1;
const MAX_MAX_ENTRIES: u32 = 5_000;
const MIN_MAX_AGE_DAYS: u32 = 1;
const MAX_MAX_AGE_DAYS: u32 = 3_650;
const MILLIS_PER_DAY: u64 = 24 * 60 * 60 * 1_000;

/// Clipboard history retention policy supplied by the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardConfig {
    pub max_entries: Option<u32>,
    pub max_age_days: Option<u32>,
}

impl ClipboardConfig {
    pub fn from_configuration(configuration: &ExtensionConfiguration) -> Result<Self, String> {
        let values = configuration.values();
        let config = Self {
            max_entries: unsigned_integer(values, MAX_ENTRIES_KEY)?,
            max_age_days: unsigned_integer(values, MAX_AGE_DAYS_KEY)?,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self
            .max_entries
            .is_some_and(|value| !(MIN_MAX_ENTRIES..=MAX_MAX_ENTRIES).contains(&value))
        {
            return Err(format!(
                "clipboard maxEntries must be between {MIN_MAX_ENTRIES} and {MAX_MAX_ENTRIES}"
            ));
        }
        if self
            .max_age_days
            .is_some_and(|value| !(MIN_MAX_AGE_DAYS..=MAX_MAX_AGE_DAYS).contains(&value))
        {
            return Err(format!(
                "clipboard maxAgeDays must be between {MIN_MAX_AGE_DAYS} and {MAX_MAX_AGE_DAYS}"
            ));
        }
        Ok(())
    }

    pub fn cutoff_millis(&self, now: u64) -> Option<u64> {
        self.max_age_days
            .map(|days| now.saturating_sub(u64::from(days).saturating_mul(MILLIS_PER_DAY)))
    }
}

fn unsigned_integer(
    values: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<Option<u32>, String> {
    if values.get(key).is_some_and(serde_json::Value::is_null) {
        return Ok(None);
    }
    values
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .map(Some)
        .ok_or_else(|| format!("clipboard configuration is missing integer {key}"))
}
