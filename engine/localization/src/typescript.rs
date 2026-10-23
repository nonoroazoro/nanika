use crate::ResourceSet;

/// Emit the package's message IDs and required arguments from its admitted default locale.
/// This is authoring information only; runtime validation still enforces safe integers,
/// argument bounds, resource ownership and formatted output limits.
pub fn typescript_message_type(resources: &ResourceSet) -> String {
    let prepared = resources.prepared();
    let contracts = &prepared.locales[&prepared.default_locale].contracts;
    let mut output = String::from(
        "// Generated from validated FTL resources. Do not edit.\n\
         export type LocalizedMessage =\n"
    );
    if contracts.is_empty() {
        output.push_str("never;\n");
        return output;
    }
    for (id, contract) in contracts {
        // Admission permits only ASCII identifier characters, so quoted keys need no escaping.
        output.push_str(&format!("| {{ message: \"{id}\"; "));
        let arguments: Vec<_> = contract
            .arguments
            .iter()
            .filter(|name| name.as_str() != "platform")
            .collect();
        if arguments.is_empty() {
            output.push_str("args?: Record<string, never>; }");
        } else {
            output.push_str("args: { ");
            for name in arguments {
                output.push_str(&format!("\"{name}\": string | number; "));
            }
            output.push_str("}; }");
        }
        output.push('\n');
    }
    output.push_str(";\n");
    output
}
