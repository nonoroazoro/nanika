use crate::{ResourceSet, SupportedLocale, validate_action_presentation};
use nanika_protocol::{LocalizedText, NavigationEffect, View};

/// Check pushed views from both action completions and view-event responses.
pub fn validate_navigation_presentation(
    effect: &NavigationEffect,
    resources: Option<&ResourceSet>
) -> Result<(), String> {
    effect.validate()?;
    if let NavigationEffect::Push { view, .. } = effect {
        validate_view_presentation(view, resources)?;
    }
    Ok(())
}

/// Validate a complete package-owned view before replacing the last accepted publication.
pub fn validate_view_presentation(
    view: &View,
    resources: Option<&ResourceSet>
) -> Result<(), String> {
    view.validate()?;
    view.visit_actions(&mut |actions| validate_action_presentation(actions, resources))?;
    view.visit_texts(&mut |field, text, maximum, allow_empty| {
        if matches!(text, LocalizedText::Literal(_)) {
            return Ok(());
        }
        let resources = resources
            .ok_or_else(|| format!("{field}: message reference requires localization resources"))?;
        for locale in [SupportedLocale::English, SupportedLocale::SimplifiedChinese] {
            for platform in ["macos", "windows"] {
                resources
                    .format(locale, text, platform, maximum, allow_empty)
                    .map_err(|error| {
                        format!("{field}: {}: {platform}: {error}", locale.as_str())
                    })?;
            }
        }
        Ok(())
    })
}
