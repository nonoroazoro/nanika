/// Read preferred UI languages in OS order. The platform API represents failed reads
/// with an empty iterator, so preserve that distinction from a valid unsupported list.
pub fn preferred_ui_languages() -> Option<Vec<String>> {
    let languages: Vec<_> = sys_locale::get_locales().collect();
    (!languages.is_empty()).then_some(languages)
}
