use crate::{GeneralSettingsField, GeneralSettingsSection};

/// The presentation contract used by both the General renderer and Settings search.
/// Keys identify controls and anchors, independently of display text or storage ownership.
pub(crate) fn sections() -> Vec<GeneralSettingsSection> {
    vec![
        GeneralSettingsSection {
            key: "launcher",
            title: "Launcher",
            fields: vec![
                GeneralSettingsField {
                    key: "launcherShortcut",
                    title: "Open launcher",
                    description: None,
                    keywords: "shortcut hotkey keyboard",
                },
                GeneralSettingsField {
                    key: "hideOnBlur",
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
                key: "theme",
                title: "Theme",
                description: None,
                keywords: "system light dark mode",
            }],
        },
        GeneralSettingsSection {
            key: "startup",
            title: "Startup",
            fields: vec![GeneralSettingsField {
                key: "launchAtLogin",
                title: "Launch at login",
                description: Some("Launch Nanika automatically when you sign in."),
                keywords: "startup boot automatic",
            }],
        },
    ]
}
