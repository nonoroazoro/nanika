use std::sync::mpsc::SyncSender;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{MainThreadMarker, NSString};

use crate::{MacMenuTarget, PlatformError, PlatformEvent};

pub(crate) struct MacNativeMenu {
    status_bar: Retained<NSStatusBar>,
    status_item: Retained<NSStatusItem>,
    _target: Retained<MacMenuTarget>,
    _menu: Retained<NSMenu>,
}

impl MacNativeMenu {
    pub(crate) fn new(events: SyncSender<PlatformEvent>) -> Result<Self, PlatformError> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            PlatformError::Message("macOS menu must be created on the main thread".to_owned())
        })?;
        let status_bar = NSStatusBar::systemStatusBar();
        let status_item = status_bar.statusItemWithLength(NSVariableStatusItemLength);
        let button = status_item.button(mtm).ok_or_else(|| {
            PlatformError::Message("macOS status item button is unavailable".to_owned())
        })?;
        button.setTitle(&NSString::from_str("N"));

        let target = MacMenuTarget::new(mtm, events);
        let menu = NSMenu::initWithTitle(NSMenu::alloc(mtm), &NSString::from_str("Nanika"));
        add_item(&menu, "Open Nanika", sel!(open:), &target);
        add_item(&menu, "Settings", sel!(settings:), &target);
        add_item(
            &menu,
            "Rescan applications",
            sel!(rescanApplications:),
            &target,
        );
        menu.addItem(&NSMenuItem::separatorItem(mtm));
        add_item(&menu, "Quit", sel!(quit:), &target);
        status_item.setMenu(Some(&menu));

        Ok(Self {
            status_bar,
            status_item,
            _target: target,
            _menu: menu,
        })
    }
}

impl Drop for MacNativeMenu {
    fn drop(&mut self) {
        self.status_bar.removeStatusItem(&self.status_item);
    }
}

fn add_item(menu: &NSMenu, title: &str, action: objc2::runtime::Sel, target: &MacMenuTarget) {
    let item = unsafe {
        menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(title),
            Some(action),
            &NSString::from_str(""),
        )
    };
    unsafe {
        item.setTarget(Some(target));
        item.setAction(Some(action));
    }
}
