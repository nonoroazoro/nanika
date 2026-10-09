use nanika_protocol::SystemAction;

/// Manifest command IDs are extension-owned, independent of host request encoding.
pub fn action_for_entry(entry_id: &str) -> Option<SystemAction> {
    Some(match entry_id {
        "lock" => SystemAction::Lock,
        "sleep" => SystemAction::Sleep,
        "displays" => SystemAction::TurnOffDisplays,
        "logout" => SystemAction::LogOut,
        "restart" => SystemAction::Restart,
        "shutdown" => SystemAction::ShutDown,
        "trash.open" => SystemAction::OpenTrash,
        "trash.empty" => SystemAction::EmptyTrash,
        _ => return None
    })
}
