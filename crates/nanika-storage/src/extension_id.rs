/// Returns whether a value is a safe lowercase reverse-DNS extension identity.
pub fn is_valid_extension_id(value: &str) -> bool {
    let mut segments = value.split('.');
    let Some(first) = segments.next() else {
        return false;
    };
    let remaining = segments.collect::<Vec<_>>();
    if remaining.is_empty() || is_windows_device_name(first) {
        return false;
    }
    std::iter::once(first)
        .chain(remaining)
        .all(is_valid_segment)
}

fn is_valid_segment(segment: &str) -> bool {
    let mut characters = segment.chars();
    matches!(characters.next(), Some('a'..='z'))
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !segment.ends_with('-')
}

fn is_windows_device_name(value: &str) -> bool {
    matches!(
        value,
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    )
}
