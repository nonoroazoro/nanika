use objc2_core_foundation::{CFArray, CFRetained, CFString, CFType};
use std::ptr::NonNull;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyInputSourceID: *const CFString;
    fn TISCreateInputSourceList(
        properties: *const std::ffi::c_void,
        include_all_installed: u8
    ) -> *mut CFArray<CFType>;
}

#[test]
fn physical_labels_follow_native_layouts_and_the_command_layer() {
    // TIS forbids concurrent calls, including from parallel Rust test workers.
    // Keep the installed-layout cases in the same test as the active-layout read.
    let labels = super::keyboard_layout_labels();
    assert_eq!(labels.len(), 36);
    assert!(labels.contains_key("KeyI"));
    assert!(labels.contains_key("Digit1"));
    assert!(
        labels
            .values()
            .all(|label| !label.is_empty() && !label.chars().any(char::is_control))
    );
    let labels = _labels("com.apple.keylayout.Dvorak");
    assert_eq!(labels["KeyD"], "E");
    assert_eq!(labels["KeyE"], ".");
    assert_eq!(labels["KeyI"], "C");
    assert_eq!(labels.len(), 36);
    assert!(
        labels
            .keys()
            .all(|key| key.starts_with("Key") || key.starts_with("Digit"))
    );
    let labels = _labels("com.apple.keylayout.DVORAK-QWERTYCMD");
    assert_eq!(labels["KeyD"], "D");
    assert_eq!(labels["KeyE"], "E");
    assert_eq!(labels["KeyI"], "I");
}

fn _labels(identifier: &str) -> std::collections::BTreeMap<String, String> {
    // Enumerate installed layouts without enabling or selecting an input source.
    let sources = NonNull::new(unsafe { TISCreateInputSourceList(std::ptr::null(), 1) })
        .map(|sources| unsafe { CFRetained::from_raw(sources) })
        .expect("installed keyboard layouts");
    let source = sources
        .iter()
        .find(|source| {
            unsafe {
                super::TISGetInputSourceProperty(&**source, kTISPropertyInputSourceID)
                    .cast::<CFString>()
                    .as_ref()
            }
            .is_some_and(|value| value.to_string() == identifier)
        })
        .expect("system keyboard layout");
    super::_physical_labels(&source)
}
