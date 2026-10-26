use crate::{MAX_EXPANDED_BYTES, ResourceSet, SupportedLocale};
use nanika_protocol::{Candidate, CandidateSubtitle, LocalizedText};

/// Admit candidate presentation before indexing. Only titles and aliases enter the search index.
/// Both languages match independently of the UI locale; inactive platform branches are validated.
pub fn candidate_search_text(
    candidate: &Candidate,
    resources: Option<&ResourceSet>,
    platform: &str
) -> Result<Vec<String>, String> {
    if !matches!(platform, "macos" | "windows") {
        return Err("candidate search requires a supported platform".into());
    }
    let mut fields = vec![("title", &candidate.title, true)];
    if let Some(CandidateSubtitle::Label(text) | CandidateSubtitle::Description(text)) =
        &candidate.subtitle
    {
        fields.push(("subtitle", text, false));
    }
    fields.extend(candidate.aliases.iter().map(|alias| ("alias", alias, true)));
    let mut values = Vec::new();
    for (field, text, searchable) in fields {
        match text {
            LocalizedText::Literal(value) => {
                if value.len() > MAX_EXPANDED_BYTES
                    || value
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                {
                    return Err(format!(
                        "{}: {field}: candidate text is oversized or contains invalid controls",
                        candidate.entry_id
                    ));
                }
                if searchable && !value.trim().is_empty() && !values.contains(value) {
                    values.push(value.clone());
                }
            }
            LocalizedText::Translated(translated) => {
                translated.validate()?;
                for value in translated.values() {
                    crate::LocaleBundle::_validate_output(value, MAX_EXPANDED_BYTES, false)
                        .map_err(|error| format!("{}: {field}: {error}", candidate.entry_id))?;
                    if searchable
                        && !value.trim().is_empty()
                        && !values.iter().any(|existing| existing == value)
                    {
                        values.push(value.to_owned());
                    }
                }
            }
            LocalizedText::Message(reference) => {
                let resources = resources.ok_or_else(|| {
                    format!(
                        "{}: {field}: message reference requires localization resources",
                        candidate.entry_id
                    )
                })?;
                let mut previous: Option<&crate::LocaleBundle> = None;
                for locale in [SupportedLocale::English, SupportedLocale::SimplifiedChinese] {
                    let bundle = resources.bundle_for(locale, text);
                    // Fallback can select the same immutable bundle for both requested locales.
                    if previous.is_some_and(|previous| std::ptr::eq(previous, bundle)) {
                        continue;
                    }
                    previous = Some(bundle);
                    let platform_dependent = bundle
                        .contracts()
                        .get(&reference.message)
                        .is_some_and(|contract| contract.arguments.contains("platform"));
                    for target in ["macos", "windows"] {
                        // Contracts include transitive references and every selector branch.
                        // Without a platform argument, the other OS has identical output.
                        if target != platform && !platform_dependent {
                            continue;
                        }
                        let value = bundle
                            .format(text, target, MAX_EXPANDED_BYTES, false)
                            .map_err(|error| {
                                format!(
                                    "{}: {field}: {}: {target}: {error}",
                                    candidate.entry_id,
                                    locale.as_str()
                                )
                            })?;
                        if searchable && target == platform && !values.contains(&value) {
                            values.push(value);
                        }
                    }
                }
            }
        }
    }
    Ok(values)
}
