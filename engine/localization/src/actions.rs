use crate::{ResourceSet, SupportedLocale};
use nanika_protocol::{Action, LocalizedText};

/// Validate package-owned action messages before accepting metadata or a runtime publication.
pub fn validate_action_presentation(
    actions: &[Action],
    resources: Option<&ResourceSet>
) -> Result<(), String> {
    nanika_protocol::validate_actions(actions)?;
    for action in actions {
        action.visit_texts(&mut |field, text, maximum| {
            if !matches!(text, LocalizedText::Message(_)) {
                return Ok(());
            }
            let resources = resources.ok_or_else(|| {
                format!(
                    "{}: {field}: message reference requires localization resources",
                    action.id
                )
            })?;
            for locale in [SupportedLocale::English, SupportedLocale::SimplifiedChinese] {
                for platform in ["macos", "windows"] {
                    resources
                        .format(locale, text, platform, maximum, false)
                        .map_err(|error| {
                            format!(
                                "{}: {field}: {}: {platform}: {error}",
                                action.id,
                                locale.as_str()
                            )
                        })?;
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}
