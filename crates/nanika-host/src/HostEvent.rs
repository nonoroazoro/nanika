use global_hotkey::GlobalHotKeyEvent;

pub(crate) enum HostEvent {
    Hotkey(GlobalHotKeyEvent),
    Activate,
}
