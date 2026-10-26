use crate::{LocaleBundle, ResourceSet, SupportedLocale};
use nanika_protocol::LocalizedText;

/// Resolve domain translations independently of package-resource fallback.
pub fn format_text(
    text: &LocalizedText,
    resources: Option<&ResourceSet>,
    locale: SupportedLocale,
    platform: &str,
    maximum_chars: usize,
    allow_empty: bool
) -> Result<String, String> {
    if !matches!(platform, "macos" | "windows") {
        return Err("Unsupported localization platform.".into());
    }
    if let LocalizedText::Message(_) = text {
        return resources
            .ok_or("message reference requires localization resources")?
            .bundle_for(locale, text)
            .format(text, platform, maximum_chars, allow_empty);
    }
    text.validate_literals(|value| {
        LocaleBundle::_validate_output(value, maximum_chars, allow_empty)
    })?;
    Ok(match text {
        LocalizedText::Literal(value) => value.clone(),
        LocalizedText::Translated(value) => value.resolve(locale.as_str()).to_owned(),
        LocalizedText::Message(_) => unreachable!()
    })
}

/// Preserve all source names for search; only package messages need UI-locale expansion.
/// Validate every supported platform branch before accepting message resources.
pub fn text_search_values(
    text: &LocalizedText,
    resources: Option<&ResourceSet>,
    platform: &str,
    maximum_chars: usize,
    validate: impl Fn(&str) -> Result<(), String>
) -> Result<Vec<String>, String> {
    if !matches!(platform, "macos" | "windows") {
        return Err("Unsupported localization platform.".into());
    }
    text.validate_literals(|value| {
        LocaleBundle::_validate_output(value, maximum_chars, false)?;
        validate(value)
    })?;
    let mut values = Vec::new();
    match text {
        LocalizedText::Literal(value) => values.push(value.clone()),
        LocalizedText::Translated(text) => {
            for value in text.values() {
                if !values.iter().any(|existing| existing == value) {
                    values.push(value.to_owned());
                }
            }
        }
        LocalizedText::Message(_) => {
            for locale in [SupportedLocale::English, SupportedLocale::SimplifiedChinese] {
                for target in ["macos", "windows"] {
                    let value = format_text(text, resources, locale, target, maximum_chars, false)?;
                    validate(&value)
                        .map_err(|error| format!("{}: {target}: {error}", locale.as_str()))?;
                    if target == platform && !values.contains(&value) {
                        values.push(value);
                    }
                }
            }
        }
    }
    Ok(values)
}
