use crate::{ResourceSet, SupportedLocale};
use nanika_protocol::{LocalizedText, OperationProgress};

/// Validate task-owned labels without changing work units or task completion.
pub fn validate_progress_presentation(
    progress: &OperationProgress,
    resources: Option<&ResourceSet>
) -> Result<(), String> {
    progress.validate()?;
    if matches!(progress.label, LocalizedText::Literal(_)) {
        return Ok(());
    }
    let resources =
        resources.ok_or("progress label: message reference requires localization resources")?;
    for locale in [SupportedLocale::English, SupportedLocale::SimplifiedChinese] {
        for platform in ["macos", "windows"] {
            let label = resources
                .format(locale, &progress.label, platform, 128, false)
                .map_err(|error| {
                    format!("progress label: {}: {platform}: {error}", locale.as_str())
                })?;
            if label.len() > 128 {
                return Err(format!(
                    "progress label: {}: {platform}: exceeds 128 bytes",
                    locale.as_str()
                ));
            }
        }
    }
    Ok(())
}
