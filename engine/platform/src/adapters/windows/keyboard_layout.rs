use std::collections::BTreeMap;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, MAPVK_VK_TO_CHAR, MAPVK_VSC_TO_VK_EX, MapVirtualKeyExW
};

/// Call on the window's event thread, whose input layout owns shortcut dispatch.
pub fn keyboard_layout_labels() -> BTreeMap<String, String> {
    let layout = unsafe { GetKeyboardLayout(0) };
    let letters = [
        0x1e, 0x30, 0x2e, 0x20, 0x12, 0x21, 0x22, 0x23, 0x17, 0x24, 0x25, 0x26, 0x32, 0x31, 0x18,
        0x19, 0x10, 0x13, 0x1f, 0x14, 0x16, 0x2f, 0x11, 0x2d, 0x15, 0x2c
    ];
    let digits = [0x0b, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a];
    letters
        .into_iter()
        .enumerate()
        .map(|(i, scan)| (format!("Key{}", (b'A' + i as u8) as char), scan))
        .chain(
            digits
                .into_iter()
                .enumerate()
                .map(|(i, scan)| (format!("Digit{i}"), scan))
        )
        .filter_map(|(code, scan)| {
            let key = unsafe { MapVirtualKeyExW(scan, MAPVK_VSC_TO_VK_EX, layout) };
            let value = unsafe { MapVirtualKeyExW(key, MAPVK_VK_TO_CHAR, layout) };
            // The high bit marks a dead key; the low word still carries its display character.
            let value = char::from_u32(value & 0xffff).filter(|value| !value.is_control())?;
            let mut label = value.to_string();
            label.make_ascii_uppercase();
            Some((code, label))
        })
        .collect()
}
