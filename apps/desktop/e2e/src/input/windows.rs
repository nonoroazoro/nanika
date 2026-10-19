use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput,
    VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_F12, VK_I, VK_LWIN, VK_MENU, VK_N, VK_OEM_COMMA, VK_RETURN,
    VK_RWIN, VK_SHIFT, VK_TAB, VK_UP
};

pub(crate) fn press(key: &str) -> Result<Vec<u16>, String> {
    let keys: &[u16] = match key {
        "launcher" => &[VK_CONTROL, VK_MENU, VK_SHIFT, VK_F12],
        "settings" => &[VK_CONTROL, VK_OEM_COMMA],
        "switch-app" => &[VK_MENU, VK_TAB],
        "escape" => &[VK_ESCAPE],
        "enter" => &[VK_RETURN],
        "down" => &[VK_DOWN],
        "up" => &[VK_UP],
        "n" => &[VK_N],
        "i" => &[VK_I],
        _ => return Err(format!("Unknown native key: {key}"))
    };
    let observed: Vec<_> = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN]
        .into_iter()
        .chain(keys.iter().copied())
        .collect();
    super::wait_for_release(&observed).map_err(|error| format!("Cannot inject {key}: {error}"))?;
    let events: Vec<_> = keys
        .iter()
        .map(|key| _event(*key, false))
        .chain(keys.iter().rev().map(|key| _event(*key, true)))
        .collect();
    let sent = unsafe {
        SendInput(
            events.len() as u32,
            events.as_ptr(),
            size_of::<INPUT>() as i32
        )
    };
    if sent != events.len() as u32 {
        let releases: Vec<_> = keys.iter().rev().map(|key| _event(*key, true)).collect();
        let released = unsafe {
            SendInput(
                releases.len() as u32,
                releases.as_ptr(),
                size_of::<INPUT>() as i32
            )
        };
        return Err(format!(
            "SendInput delivered {sent}/{} events; cleanup delivered {released}/{} releases. Check process integrity levels and input permissions.",
            events.len(),
            releases.len()
        ));
    }
    Ok(keys.to_vec())
}

pub(crate) fn is_down(key: u16) -> bool {
    unsafe { GetAsyncKeyState(i32::from(key)) < 0 }
}

fn _event(key: u16, released: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                dwFlags: if released { KEYEVENTF_KEYUP } else { 0 },
                ..Default::default()
            }
        }
    }
}
