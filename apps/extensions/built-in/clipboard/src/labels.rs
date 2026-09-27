/// Escape controls for a bounded label, preserving raw content in the database.
pub(crate) fn display_label(value: &str, maximum: usize) -> String {
    let mut output = String::new();
    let mut count = 0;
    for character in value.chars() {
        let escaped = character.is_control().then(|| character.escape_default());
        for next in escaped
            .into_iter()
            .flatten()
            .chain((!character.is_control()).then_some(character))
        {
            if count == maximum {
                output.push_str("...");
                return output;
            }
            output.push(next);
            count += 1;
        }
    }
    // Whitespace-only native names are data, but an invisible label cannot identify a row.
    if output.trim().is_empty() {
        format!("{output:?}")
    } else {
        output
    }
}
