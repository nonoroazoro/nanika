use crate::ProcessLauncher;
use nanika_protocol::{LaunchArguments, LaunchDescriptor, SystemAction};
use objc2::rc::autoreleasepool;
use objc2_app_kit::NSWorkspace;
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventTapLocation, CGPreflightPostEventAccess, CGRequestPostEventAccess,
};
use objc2_foundation::{NSString, NSURL};

pub(crate) fn executor() -> std::io::Result<impl Fn(SystemAction) -> Result<(), String> + Send> {
    let launcher = ProcessLauncher::spawn()?;
    Ok(move |action| _execute(action, &launcher))
}

fn _execute(action: SystemAction, launcher: &ProcessLauncher) -> Result<(), String> {
    autoreleasepool(|_| match action {
        SystemAction::Lock => _lock(),
        SystemAction::Sleep => _launch(launcher, "/usr/bin/pmset", &["sleepnow"]),
        SystemAction::TurnOffDisplays => _launch(launcher, "/usr/bin/pmset", &["displaysleepnow"]),
        SystemAction::LogOut => _script(launcher, "tell application \"System Events\" to log out"),
        SystemAction::Restart => _script(launcher, "tell application \"System Events\" to restart"),
        SystemAction::ShutDown => {
            _script(launcher, "tell application \"System Events\" to shut down")
        }
        SystemAction::OpenTrash => {
            let home =
                directories::BaseDirs::new().ok_or("Could not resolve the home directory.")?;
            let path = home.home_dir().join(".Trash");
            let path = path.to_str().ok_or("Trash path is not valid UTF-8.")?;
            let url = NSURL::fileURLWithPath(&NSString::from_str(path));
            if NSWorkspace::sharedWorkspace().openURL(&url) {
                Ok(())
            } else {
                Err("Finder could not open Trash.".to_owned())
            }
        }
        SystemAction::EmptyTrash => {
            // Finder owns all mounted-volume trash and its permissions. Never enumerate ~/.Trash.
            _script(launcher, "tell application \"Finder\" to empty trash")
        }
    })
}

fn _lock() -> Result<(), String> {
    if !CGPreflightPostEventAccess() {
        CGRequestPostEventAccess();
        return Err("Allow Nanika in System Settings > Privacy & Security > Accessibility, then run Lock Screen again.".to_owned());
    }
    // Public Control-Command-Q shortcut. Construct both events before posting either.
    let down =
        CGEvent::new_keyboard_event(None, 12, true).ok_or("Could not create lock key event.")?;
    let up =
        CGEvent::new_keyboard_event(None, 12, false).ok_or("Could not create lock key release.")?;
    let flags = CGEventFlags::MaskControl | CGEventFlags::MaskCommand;
    CGEvent::set_flags(Some(&down), flags);
    CGEvent::set_flags(Some(&up), flags);
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&down));
    CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&up));
    Ok(())
}

fn _script(launcher: &ProcessLauncher, source: &str) -> Result<(), String> {
    // The interpreter owns AppleScript's main thread. Only process launch is acknowledged.
    _launch(
        launcher,
        "/usr/bin/osascript",
        &["-l", "AppleScript", "-e", source],
    )
}

fn _launch(launcher: &ProcessLauncher, program: &str, arguments: &[&str]) -> Result<(), String> {
    launcher.launch(LaunchDescriptor::Program {
        program: program.to_owned(),
        arguments: LaunchArguments::Structured {
            values: arguments.iter().map(|value| (*value).to_owned()).collect(),
        },
        working_directory: None,
    })
}
