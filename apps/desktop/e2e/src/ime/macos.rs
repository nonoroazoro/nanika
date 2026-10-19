use std::ffi::c_void;
use std::ptr::NonNull;

use objc2_app_kit::NSWorkspace;
use objc2_core_foundation::{CFArray, CFBoolean, CFRetained, CFString, CFType};

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    static kTISPropertyInputSourceID: *const CFString;
    static kTISPropertyInputSourceLanguages: *const CFString;
    static kTISPropertyInputSourceIsSelectCapable: *const CFString;
    fn TISCopyCurrentKeyboardInputSource() -> *mut CFType;
    fn TISCreateInputSourceList(
        properties: *const c_void,
        include_all_installed: u8
    ) -> *mut CFArray<CFType>;
    fn TISGetInputSourceProperty(source: *const CFType, property: *const CFString)
    -> *const c_void;
    fn TISSelectInputSource(source: *const CFType) -> i32;
}

pub(crate) fn run(pid: u32) -> Result<(), String> {
    if NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .is_none_or(|app| app.processIdentifier() as u32 != pid)
    {
        return Err("IME verification requires the owned App to be frontmost".into());
    }
    let original = NonNull::new(unsafe { TISCopyCurrentKeyboardInputSource() })
        .map(|source| unsafe { CFRetained::from_raw(source) })
        .ok_or("Could not capture the current input source")?;
    // Select only an already-enabled input method; verification must not install or enable one.
    let sources = NonNull::new(unsafe { TISCreateInputSourceList(std::ptr::null(), 0) })
        .map(|sources| unsafe { CFRetained::from_raw(sources) })
        .ok_or("Could not enumerate enabled input sources")?;
    let selected = if _supports_chinese(&original) {
        original.clone()
    } else {
        sources
            .iter()
            .find(|source| _supports_chinese(source))
            .ok_or(
                "IME capability unavailable: this scenario requires an enabled Chinese input method"
            )?
    };
    let identifier = unsafe {
        TISGetInputSourceProperty(&*selected, kTISPropertyInputSourceID)
            .cast::<CFString>()
            .as_ref()
    }
    .ok_or("The selected input source has no identifier")?;
    eprintln!("IME source: {identifier}");
    let result = _select(&selected).and_then(|()| super::_wait_for_owner());
    _select(&original).map_err(|error| format!("Could not restore the input source: {error}"))?;
    result
}

fn _supports_chinese(source: &CFType) -> bool {
    // TextInputSources.h defines these properties as CFBoolean and CFArray<CFString>.
    let selectable = unsafe {
        TISGetInputSourceProperty(source, kTISPropertyInputSourceIsSelectCapable)
            .cast::<CFBoolean>()
            .as_ref()
    };
    let languages = unsafe {
        TISGetInputSourceProperty(source, kTISPropertyInputSourceLanguages)
            .cast::<CFArray<CFString>>()
            .as_ref()
    };
    selectable.is_some_and(CFBoolean::value)
        && languages.is_some_and(|languages| {
            languages.iter().any(|language| {
                let language = language.to_string();
                language == "zh" || language.starts_with("zh-")
            })
        })
}

fn _select(source: &CFType) -> Result<(), String> {
    let status = unsafe { TISSelectInputSource(source) };
    if status != 0 {
        return Err(format!("TISSelectInputSource failed with status {status}"));
    }
    Ok(())
}
