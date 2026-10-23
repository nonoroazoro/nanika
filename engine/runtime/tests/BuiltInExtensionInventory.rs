use crate::BuiltInExtensionInventory;

#[test]
fn every_built_in_uses_the_ordinary_manifest_contract() {
    let inventory = BuiltInExtensionInventory::parse(nanika_builtin_inventory::MANIFESTS)
        .expect("built-in manifests");
    assert_eq!(
        inventory.extensions.len(),
        nanika_builtin_inventory::MANIFESTS.len()
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
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/extensions/built-in");
    let directories: Vec<String> =
        serde_json::from_slice(&std::fs::read(root.join("inventory.json")).unwrap()).unwrap();
    for (extension, directory) in inventory.extensions.into_iter().zip(directories) {
        let installed = nanika_extension_package::InstalledExtension::from_manifest(
            extension.manifest,
            "unused".into(),
            root.join(directory),
            true
        )
        .unwrap();
        assert!(matches!(
            installed.name,
            nanika_protocol::LocalizedText::Message(_)
        ));
        assert!(installed.localization.is_some());
        assert_eq!(installed.name_search_values.len(), 2);
    }
}

#[test]
fn host_inventory_accepts_new_identities_and_rejects_duplicates() {
    assert!(
        BuiltInExtensionInventory::parse(&[])
            .unwrap()
            .extensions
            .is_empty()
    );
    let source = nanika_builtin_inventory::MANIFESTS[0]
        .replace("com.nanika.application", "org.example.custom");
    let inventory = BuiltInExtensionInventory::parse(&[&source]).unwrap();
    assert_eq!(inventory.extensions[0].manifest.id, "org.example.custom");
    assert!(BuiltInExtensionInventory::parse(&[&source, &source]).is_err());
}
