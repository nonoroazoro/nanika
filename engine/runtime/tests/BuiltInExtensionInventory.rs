use std::collections::HashSet;

use crate::BuiltInExtensionInventory;

#[test]
fn every_built_in_uses_the_ordinary_manifest_contract() {
    let sources = [
        include_str!("../../../apps/extensions/built-in/application/manifest.jsonc"),
        include_str!("../../../apps/extensions/built-in/command/manifest.jsonc"),
        include_str!("../../../apps/extensions/built-in/script/manifest.jsonc"),
        include_str!("../../../apps/extensions/built-in/calculator/manifest.jsonc"),
        include_str!("../../../apps/extensions/built-in/clipboard/manifest.jsonc"),
    ];
    let inventory = BuiltInExtensionInventory::parse(&sources).expect("built-in manifests");
    assert_eq!(inventory.extensions.len(), 5);
    let identifiers = inventory
        .extensions
        .iter()
        .map(|extension| extension.manifest.id.as_str())
        .collect::<HashSet<_>>();
    assert_eq!(
        identifiers,
        nanika_foundation::BUILTIN_EXTENSION_IDS
            .into_iter()
            .collect::<HashSet<_>>()
    );
    let clipboard = inventory
        .extensions
        .iter()
        .find(|extension| extension.manifest.id == "com.nanika.clipboard")
        .expect("clipboard manifest");
    assert!(clipboard.manifest.contributes.commands.is_empty());
    assert_eq!(clipboard.manifest.contributes.views.len(), 1);
    assert_eq!(
        clipboard.manifest.contributes.views[0].id,
        "clipboard.history"
    );
    let calculator = inventory
        .extensions
        .iter()
        .find(|extension| extension.manifest.id == "com.nanika.calculator")
        .expect("calculator manifest");
    assert!(calculator.manifest.contributes.commands.is_empty());
    assert!(calculator.manifest.contributes.views.is_empty());
    assert!(calculator.manifest.contributes.root_search.is_some());
}
