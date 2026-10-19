use nanika_protocol::{ExtensionConfiguration, IntegerLimit};

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
    pub max_age_days: Option<u32>
}

impl ClipboardConfig {
    pub fn from_configuration(configuration: &ExtensionConfiguration) -> Result<Self, String> {
        let values = configuration.values();
        let config = Self {
            max_entries: _integer_limit(values, MAX_ENTRIES_KEY, MIN_MAX_ENTRIES, MAX_MAX_ENTRIES)?,
            max_age_days: _integer_limit(
                values,
                MAX_AGE_DAYS_KEY,
                MIN_MAX_AGE_DAYS,
                MAX_MAX_AGE_DAYS
            )?
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

fn _integer_limit(
    values: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
    minimum: u32,
    maximum: u32
) -> Result<Option<u32>, String> {
    let limit: IntegerLimit = serde_json::from_value(
        values
            .get(key)
            .cloned()
            .ok_or_else(|| format!("missing integer limit {key}"))?
    )
    .map_err(|error| format!("invalid integer limit {key}: {error}"))?;
    // Validate the remembered value even when the effective limit is unlimited.
    let value = u32::try_from(limit.value)
        .ok()
        .filter(|value| (minimum..=maximum).contains(value))
        .ok_or_else(|| format!("{key} must be between {minimum} and {maximum}"))?;
    Ok(limit.effective().map(|_| value))
}
