use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Export only supported UI languages from the same pinned ICU4X data used by Rust matching.
/// WebView CLDR versions must not decide which domain name the product displays.
pub(super) fn write(path: &Path) {
    let resources = crate::host_resources::resources()
        .expect("validate host locales")
        .prepared();
    let languages = resources
        .locales
        .keys()
        .map(|locale| locale.as_str().split('-').next().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    let common = icu_locale::fallback::provider::Baked::SINGLETON_LOCALE_LIKELY_SUBTAGS_LANGUAGE_V1;
    let extended = icu_locale::provider::Baked::SINGLETON_LOCALE_LIKELY_SUBTAGS_EXTENDED_V1;
    let mut result = BTreeMap::new();
    for language in languages {
        let mut locale = language.parse::<icu_locale::LanguageIdentifier>().unwrap();
        icu_locale::LocaleExpander::new_extended().maximize(&mut locale);
        let script = locale
            .script
            .expect("UI language has a likely script")
            .to_string();
        let region = locale
            .region
            .expect("UI language has a likely region")
            .to_string();
        let mut scripts = BTreeMap::new();
        let mut regions = BTreeMap::new();
        for ((source, key), value) in common
            .language_script
            .iter_copied()
            .chain(extended.language_script.iter_copied())
        {
            if source.try_into_tinystr().unwrap().as_str() == language {
                scripts
                    .entry(key.try_into_tinystr().unwrap().to_string())
                    .or_insert_with(|| value.to_string());
            }
        }
        for ((source, key), value) in common
            .language_region
            .iter_copied()
            .chain(extended.language_region.iter_copied())
        {
            if source.try_into_tinystr().unwrap().as_str() == language {
                regions
                    .entry(key.try_into_tinystr().unwrap().to_string())
                    .or_insert_with(|| value.to_string());
            }
        }
        scripts.retain(|_, value| value != &region);
        regions.retain(|_, value| value != &script);
        result.insert(
            language,
            serde_json::json!({"script":script,"region":region,"scripts":scripts,"regions":regions})
        );
    }
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, format!(
        "// Generated from the pinned Rust ICU4X data for supported UI languages. Do not edit.\nexport const localeMatching: Readonly<Record<string, {{ readonly script: string; readonly region: string; readonly scripts: Readonly<Record<string, string>>; readonly regions: Readonly<Record<string, string>>; }}>> = {};\n",
        serde_json::to_string(&result).unwrap()
    )).unwrap();
}
