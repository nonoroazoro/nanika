use global_hotkey::GlobalHotKeyEvent;
use nanika_platform::PlatformEvent;

pub(crate) enum HostEvent {
    Hotkey(GlobalHotKeyEvent),
    Platform(PlatformEvent),
}
