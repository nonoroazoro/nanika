use nanika_extension_package::{ExtensionActivation, InstalledExtension, parse_extension_manifest};
use nanika_extension_system::action_for_entry;
use nanika_protocol::{ActionInvocation, SystemAction};

#[test]
fn static_manifest_covers_exactly_the_native_operations_with_confirmation() {
    let manifest = parse_extension_manifest(include_str!("../manifest.jsonc")).unwrap();
    assert_eq!(manifest.activation, ExtensionActivation::OnDemand);
    assert!(manifest.contributes.root_search.is_none());
    assert!(manifest.contributes.views.is_empty());
    assert_eq!(manifest.contributes.commands.len(), 8);
    let mut covered = std::collections::HashSet::new();
    for command in &manifest.contributes.commands {
        let action = action_for_entry(&command.command).expect("implemented command");
        assert!(covered.insert(action.permission()));
        assert!(
            manifest
                .permissions
                .iter()
                .any(|p| p == action.permission())
        );
        let destructive = matches!(
            action,
            SystemAction::LogOut
                | SystemAction::Restart
                | SystemAction::ShutDown
                | SystemAction::EmptyTrash
        );
        assert_eq!(
            command.action.allows_invocation(ActionInvocation::Default),
            !destructive
        );
        assert_eq!(
            command.action.allows_invocation(ActionInvocation::Explicit),
            !destructive
        );
        assert!(
            command
                .action
                .allows_invocation(ActionInvocation::Confirmed)
        );
    }
    assert_eq!(covered.len(), manifest.permissions.len());
    assert!(action_for_entry("codex").is_none());
}

#[test]
fn logout_and_trash_copy_uses_the_receiving_platform_in_both_languages() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let extension = InstalledExtension::from_manifest(
        parse_extension_manifest(include_str!("../manifest.jsonc")).unwrap(),
        root.join("test-program"),
        root,
        true
    )
    .unwrap();
    let resources = extension.localization.unwrap();
    for (locale, platform, logout, title, trash) in [
        (
            "en",
            "macos",
            "Log Out",
            "Log Out?",
            "Permanently delete all items in Trash"
        ),
        (
            "en",
            "windows",
            "Sign out",
            "Sign out?",
            "Permanently delete all items in the Recycle Bin"
        ),
        (
            "zh-Hans",
            "macos",
            "退出登录",
            "退出登录？",
            "永久删除废纸篓中的所有项目"
        ),
        (
            "zh-Hans",
            "windows",
            "注销",
            "注销？",
            "永久删除回收站中的所有项目"
        )
    ] {
        let locale = serde_json::from_value(serde_json::json!(locale)).unwrap();
        let logout_command = extension
            .contributes
            .commands
            .iter()
            .find(|command| command.command == "logout")
            .unwrap();
        let trash_command = extension
            .contributes
            .commands
            .iter()
            .find(|command| command.command == "trash.empty")
            .unwrap();
        for (text, expected) in [
            (&logout_command.action.title, logout),
            (
                logout_command.action.confirmation_title.as_ref().unwrap(),
                title
            ),
            (&trash_command.description, trash)
        ] {
            assert_eq!(
                resources
                    .format(locale, text, platform, 2048, false)
                    .unwrap(),
                expected
            );
        }
    }
}
