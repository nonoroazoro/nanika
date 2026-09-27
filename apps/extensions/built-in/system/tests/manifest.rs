use nanika_extension_package::{ExtensionActivation, parse_extension_manifest};
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
