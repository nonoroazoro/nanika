use crate::{GeneralSettingsField, GeneralSettingsFieldKey, GeneralSettingsSection};

/// The presentation contract used by both the General renderer and Settings search.
/// Keys identify controls and anchors, independently of display text or storage ownership.
pub(crate) fn sections() -> Vec<GeneralSettingsSection> {
    vec![
        GeneralSettingsSection {
            key: "launcher",
            title: "Launcher",
            fields: vec![
                GeneralSettingsField {
                    key: GeneralSettingsFieldKey::LauncherShortcut,
                    title: "Open launcher",
                    description: None,
                    keywords: "shortcut hotkey keyboard",
                },
                GeneralSettingsField {
                    key: GeneralSettingsFieldKey::HideOnBlur,
                    title: "Hide when focus is lost",
                    description: None,
                    keywords: "dismiss blur",
                },
            ],
        },
        GeneralSettingsSection {
            key: "appearance",
            title: "Appearance",
            fields: vec![GeneralSettingsField {
                key: GeneralSettingsFieldKey::Theme,
                title: "Theme",
                description: None,
                keywords: "system light dark mode",
            }],
        },
        GeneralSettingsSection {
            key: "startup",
            title: "Startup",
            fields: vec![GeneralSettingsField {
                key: GeneralSettingsFieldKey::LaunchAtLogin,
                title: "Launch at login",
                description: Some("Launch Nanika automatically when you sign in."),
                keywords: "startup boot automatic",
            }],
        },
    ]
}
