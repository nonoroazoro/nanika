use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    CGPreflightPostEventAccess
};

// IOLLEvent.h left-device flags distinguish left/right modifier state in FlagsChanged events.
const LEFT_CONTROL: CGEventFlags =
    CGEventFlags::from_bits_retain(CGEventFlags::MaskControl.bits() | 0x01);
const LEFT_SHIFT: CGEventFlags =
    CGEventFlags::from_bits_retain(CGEventFlags::MaskShift.bits() | 0x02);
const LEFT_COMMAND: CGEventFlags =
    CGEventFlags::from_bits_retain(CGEventFlags::MaskCommand.bits() | 0x08);
const LEFT_OPTION: CGEventFlags =
    CGEventFlags::from_bits_retain(CGEventFlags::MaskAlternate.bits() | 0x20);

pub(crate) fn press(key: &str) -> Result<Vec<u16>, String> {
    eprintln!(
        "Native input {key}: frontmost PID {:?}",
        objc2_app_kit::NSWorkspace::sharedWorkspace()
            .frontmostApplication()
            .map(|app| app.processIdentifier())
    );
    let (code, modifiers): (u16, &[(u16, CGEventFlags)]) = match key {
        "launcher" => (
            111,
            &[(59, LEFT_CONTROL), (58, LEFT_OPTION), (56, LEFT_SHIFT)]
        ),
        "settings" => (43, &[(55, LEFT_COMMAND)]),
        "switch-app" => (48, &[(55, LEFT_COMMAND)]),
        "escape" => (53, &[]),
        "enter" => (36, &[]),
        "primary-enter" => (36, &[(55, LEFT_COMMAND)]),
        "down" => (125, &[]),
        "up" => (126, &[]),
        "n" => (45, &[]),
        "i" => (34, &[]),
        _ => return Err(format!("Unknown native key: {key}"))
    };
    if !CGPreflightPostEventAccess() {
        return Err("Native input requires Accessibility permission for the E2E executable or its terminal.".into());
    }
    // Wait for existing input to finish; never release another sender's keys.
    super::wait_for_release(&[54, 55, 56, 60, 58, 61, 59, 62, code])
        .map_err(|error| format!("Cannot inject {key}: {error}"))?;
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .ok_or("Could not allocate keyboard event source")?;
    let mut flags = CGEventSource::flags_state(CGEventSourceStateID::CombinedSessionState)
        & CGEventFlags::MaskAlphaShift;
    let mut events = Vec::with_capacity(modifiers.len() * 2 + 2);
    for &(modifier, flag) in modifiers {
        flags |= flag;
        let event = CGEvent::new_keyboard_event(Some(&source), modifier, true)
            .ok_or("Could not allocate modifier press")?;
        CGEvent::set_flags(Some(&event), flags);
        events.push(event);
    }
    for pressed in [true, false] {
        let event = CGEvent::new_keyboard_event(Some(&source), code, pressed)
            .ok_or("Could not allocate key event")?;
        // Function keys carry Fn independently of the chord's modifier keys.
        let key_flags = if code == 111 {
            flags | CGEventFlags::MaskSecondaryFn
        } else {
            flags
        };
        CGEvent::set_flags(Some(&event), key_flags);
        events.push(event);
    }
    for &(modifier, flag) in modifiers.iter().rev() {
        flags.remove(flag);
        let event = CGEvent::new_keyboard_event(Some(&source), modifier, false)
            .ok_or("Could not allocate modifier release")?;
        CGEvent::set_flags(Some(&event), flags);
        events.push(event);
    }
    // Construct every release before posting any press so allocation failure cannot leave a modifier held.
    for event in events {
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
    }
    Ok(modifiers
        .iter()
        .map(|(code, _)| *code)
        .chain([code])
        .collect())
}

pub(crate) fn is_down(key: u16) -> bool {
    CGEventSource::key_state(CGEventSourceStateID::CombinedSessionState, key)
}
