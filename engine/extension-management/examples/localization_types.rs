use nanika_extension_package::load_extension_manifest;
use nanika_localization::{ResourceSet, typescript_message_type};
use std::path::PathBuf;

/// Authoring command only. It validates resources without starting an extension or selecting a locale.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(
        arguments
            .next()
            .ok_or("Expected an extension package directory")?
    );
    if arguments.next().is_some() {
        return Err("Expected exactly one extension package directory".into());
    }
    let manifest = load_extension_manifest(root.join("manifest.jsonc"))?;
    let declaration = manifest
        .localization
        .ok_or("The package does not declare localization resources")?;
    let resources = ResourceSet::load(&root, &declaration, false)
        .map_err(|error| format!("{}: {error}", manifest.id))?;
    print!("{}", typescript_message_type(&resources));
    Ok(())
}
