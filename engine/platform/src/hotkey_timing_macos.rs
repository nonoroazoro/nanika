use std::ffi::c_void;
use std::time::Duration;

type EventHandlerCallRef = *mut c_void;
type EventHandlerRef = *mut c_void;
type EventRef = *mut c_void;
type EventTargetRef = *mut c_void;
type OSStatus = i32;

const EVENT_CLASS_KEYBOARD: u32 = 1_801_812_322;
const EVENT_HOTKEY_PRESSED: u32 = 5;
const EVENT_HOTKEY_RELEASED: u32 = 6;
const EVENT_PARAM_DIRECT_OBJECT: u32 = 757_935_405;
const EVENT_TYPE_HOTKEY_ID: u32 = 1_751_860_796;
const NO_ERROR: OSStatus = 0;

#[repr(C, packed(2))]
struct EventHotkeyId {
    signature: u32,
    id: u32,
}

#[repr(C, packed(2))]
struct EventTypeSpec {
    event_class: u32,
    event_kind: u32,
}

type EventHandler =
    Option<unsafe extern "C" fn(EventHandlerCallRef, EventRef, *mut c_void) -> OSStatus>;

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn CallNextEventHandler(next_handler: EventHandlerCallRef, event: EventRef) -> OSStatus;
    fn GetEventDispatcherTarget() -> EventTargetRef;
    fn GetCurrentEvent() -> EventRef;
    fn GetCurrentEventTime() -> f64;
    fn GetEventKind(event: EventRef) -> u32;
    fn GetEventParameter(
        event: EventRef,
        name: u32,
        desired_type: u32,
        actual_type: *mut u32,
        buffer_size: usize,
        actual_size: *mut usize,
        data: *mut c_void,
    ) -> OSStatus;
    fn GetEventTime(event: EventRef) -> f64;
    fn InstallEventHandler(
        target: EventTargetRef,
        handler: EventHandler,
        event_type_count: usize,
        event_types: *const EventTypeSpec,
        user_data: *mut c_void,
        handler_ref: *mut EventHandlerRef,
    ) -> OSStatus;
    fn RemoveEventHandler(handler: EventHandlerRef) -> OSStatus;
}

pub(crate) fn current_delivery_delay() -> Option<Duration> {
    let event = unsafe { GetCurrentEvent() };
    if event.is_null() {
        return None;
    }
    let event_time = unsafe { GetEventTime(event) };
    let current_time = unsafe { GetCurrentEventTime() };
    (current_time >= event_time).then(|| Duration::from_secs_f64(current_time - event_time))
}

pub(crate) fn install() -> Option<*mut c_void> {
    let event_types = [
        EventTypeSpec {
            event_class: EVENT_CLASS_KEYBOARD,
            event_kind: EVENT_HOTKEY_PRESSED,
        },
        EventTypeSpec {
            event_class: EVENT_CLASS_KEYBOARD,
            event_kind: EVENT_HOTKEY_RELEASED,
        },
    ];
    let mut handler = std::ptr::null_mut();
    let status = unsafe {
        InstallEventHandler(
            GetEventDispatcherTarget(),
            Some(observe_hotkey),
            event_types.len(),
            event_types.as_ptr(),
            std::ptr::null_mut(),
            &mut handler,
        )
    };
    (status == NO_ERROR && !handler.is_null()).then_some(handler)
}

pub(crate) fn uninstall(handle: *mut c_void) {
    if !handle.is_null() {
        unsafe {
            RemoveEventHandler(handle);
        }
    }
}

unsafe extern "C" fn observe_hotkey(
    next_handler: EventHandlerCallRef,
    event: EventRef,
    _user_data: *mut c_void,
) -> OSStatus {
    let event_kind = unsafe { GetEventKind(event) };
    if matches!(event_kind, EVENT_HOTKEY_PRESSED | EVENT_HOTKEY_RELEASED) {
        let mut hotkey = EventHotkeyId {
            signature: 0,
            id: 0,
        };
        let status = unsafe {
            GetEventParameter(
                event,
                EVENT_PARAM_DIRECT_OBJECT,
                EVENT_TYPE_HOTKEY_ID,
                std::ptr::null_mut(),
                size_of::<EventHotkeyId>(),
                std::ptr::null_mut(),
                (&mut hotkey as *mut EventHotkeyId).cast(),
            )
        };
        if status == NO_ERROR {
            let event_time = unsafe { GetEventTime(event) };
            let current_time = unsafe { GetCurrentEventTime() };
            let delivery_seconds = (current_time - event_time).max(0.0);
            let delivery = delivery_seconds
                .is_finite()
                .then(|| Duration::from_secs_f64(delivery_seconds));
            if let Some(delivery) = delivery {
                crate::record_hotkey_delivery(hotkey.id, delivery);
            }
        }
    }
    unsafe { CallNextEventHandler(next_handler, event) }
}
