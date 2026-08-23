use crate::ProjectIdentity;

/// Product display name.
pub const PRODUCT_NAME: &str = "Nanika";

/// Current pre-1.0 identity baseline.
pub const PROJECT_IDENTITY: ProjectIdentity = ProjectIdentity {
    qualifier: "com",
    organization: "nanika",
    application: "nanika",
    bundle_id: "com.nanika.nanika",
};

pub const APPLICATION_EXTENSION_ID: &str = "com.nanika.application";
pub const COMMAND_EXTENSION_ID: &str = "com.nanika.command";
pub const SCRIPT_EXTENSION_ID: &str = "com.nanika.script";
pub const CALCULATOR_EXTENSION_ID: &str = "com.nanika.calculator";
pub const CLIPBOARD_EXTENSION_ID: &str = "com.nanika.clipboard";

/// Extension IDs reserved for the default distribution.
pub const BUILTIN_EXTENSION_IDS: [&str; 5] = [
    APPLICATION_EXTENSION_ID,
    COMMAND_EXTENSION_ID,
    SCRIPT_EXTENSION_ID,
    CALCULATOR_EXTENSION_ID,
    CLIPBOARD_EXTENSION_ID,
];
