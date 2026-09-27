use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};

pub(super) struct RuntimeApartment;
impl RuntimeApartment {
    pub(super) fn new() -> windows::core::Result<Self> {
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED)?;
        }
        Ok(Self)
    }
}
impl Drop for RuntimeApartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}
