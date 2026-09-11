/// Return the operating system's preferred locale as a BCP 47 language tag.
pub fn system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en-US".to_owned())
}
