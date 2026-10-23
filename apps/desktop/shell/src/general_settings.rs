use crate::{GeneralSettingsField, GeneralSettingsFieldKey, GeneralSettingsSection};

/// The presentation contract used by both the General renderer and Settings search.
/// Keys identify controls and anchors, independently of display text or storage ownership.
pub(crate) fn sections() -> Vec<GeneralSettingsSection> {
    vec![
        GeneralSettingsSection {
            key: "launcher",
            title: nanika_protocol::message("settings-launcher", []),
            fields: vec![
                GeneralSettingsField {
                    key: GeneralSettingsFieldKey::LauncherShortcut,
                    title: nanika_protocol::message("settings-open-launcher", []),
                    description: None,
                    keywords: "shortcut hotkey keyboard"
                },
                GeneralSettingsField {
                    key: GeneralSettingsFieldKey::HideOnBlur,
                    title: nanika_protocol::message("settings-hide-on-blur", []),
                    description: None,
                    keywords: "dismiss blur"
                },
            ]
        },
        GeneralSettingsSection {
            key: "appearance",
            title: nanika_protocol::message("settings-appearance", []),
            fields: vec![
                GeneralSettingsField {
                    key: GeneralSettingsFieldKey::Theme,
                    title: nanika_protocol::message("settings-theme", []),
                    description: None,
                    keywords: "system light dark mode"
                },
                GeneralSettingsField {
                    key: GeneralSettingsFieldKey::Language,
                    title: nanika_protocol::message("settings-language", []),
                    description: None,
                    keywords: "language locale English Chinese 简体中文 语言"
                },
            ]
        },
        GeneralSettingsSection {
            key: "startup",
            title: nanika_protocol::message("settings-startup", []),
            fields: vec![GeneralSettingsField {
                key: GeneralSettingsFieldKey::LaunchAtLogin,
                title: nanika_protocol::message("settings-launch-at-login", []),
                description: Some(nanika_protocol::message(
                    "settings-launch-at-login-description",
                    []
                )),
                keywords: "startup boot automatic"
            }]
        },
    ]
}
