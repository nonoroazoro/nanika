use nanika_extension_package::ExtensionManifest;

/// A validated ordinary manifest selected by the host-owned built-in inventory.
#[derive(Debug, Clone)]
pub struct BuiltInExtension {
    pub manifest: ExtensionManifest,
    pub binary_name: String,
}
