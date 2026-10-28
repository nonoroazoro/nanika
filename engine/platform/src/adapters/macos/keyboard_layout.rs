use objc2_core_foundation::{CFRetained, CFString, CFType};

#[cfg(test)]
#[path = "../../../tests/adapters/macos/keyboard_layout.rs"]
mod tests;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::ptr::NonNull;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyUnicodeKeyLayoutData: *const CFString;
    fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut CFType;
    fn TISGetInputSourceProperty(source: *const CFType, property: *const CFString)
    -> *const c_void;
    fn LMGetKbdType() -> u8;
    fn UCKeyTranslate(
        layout: *const c_void,
        key: u16,
        action: u16,
        modifiers: u32,
        keyboard: u32,
        options: u32,
        dead: *mut u32,
        maximum: usize,
        length: *mut usize,
        output: *mut u16
    ) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
}

/// Read physical-key labels in the Command layer without changing the input source.
/// Call on the window's event thread; TIS forbids concurrent access.
pub fn keyboard_layout_labels() -> BTreeMap<String, String> {
    let Some(source) = NonNull::new(unsafe { TISCopyCurrentKeyboardLayoutInputSource() }) else {
        return BTreeMap::new();
    };
    let source = unsafe { CFRetained::from_raw(source) };
    _physical_labels(&source)
}

fn _physical_labels(source: &CFType) -> BTreeMap<String, String> {
    let data = unsafe { TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData) };
    if data.is_null() {
        return BTreeMap::new();
    }
    let layout = unsafe { CFDataGetBytePtr(data) };
    if layout.is_null() {
        return BTreeMap::new();
    }
    let letters = [
        0, 11, 8, 2, 14, 3, 5, 4, 34, 38, 40, 37, 46, 45, 31, 35, 12, 15, 1, 17, 32, 9, 13, 7, 16,
        6
    ];
    let digits = [29, 18, 19, 20, 21, 23, 22, 26, 28, 25];
    letters
        .into_iter()
        .enumerate()
        .map(|(i, key)| (format!("Key{}", (b'A' + i as u8) as char), key))
        .chain(
            digits
                .into_iter()
                .enumerate()
                .map(|(i, key)| (format!("Digit{i}"), key))
        )
        .filter_map(|(code, key)| {
            let mut output = [0_u16; 8];
            let mut length = 0;
            let mut dead = 0;
            // All action chords include Command. Translate its actual layout layer,
            // including layouts whose Command layer differs from ordinary typing.
            // UCKeyTranslate takes the EventRecord modifier bits shifted right by 8.
            let status = unsafe {
                UCKeyTranslate(
                    layout.cast(),
                    key,
                    3,
                    1,
                    u32::from(LMGetKbdType()),
                    1,
                    &mut dead,
                    output.len(),
                    &mut length,
                    output.as_mut_ptr()
                )
            };
            if status != 0 || length == 0 || length > output.len() {
                return None;
            }
            let mut label = String::from_utf16(&output[..length]).ok()?;
            label.make_ascii_uppercase();
            (!label.chars().any(char::is_control)).then_some((code, label))
        })
        .collect()
}
