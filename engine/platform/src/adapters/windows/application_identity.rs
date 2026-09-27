use windows::{
    Win32::{
        Foundation::ERROR_INSUFFICIENT_BUFFER,
        Storage::Packaging::Appx::{
            APPLICATION_USER_MODEL_ID_MAX_LENGTH, ParseApplicationUserModelId,
        },
    },
    core::PCWSTR,
};

pub(super) fn validate(id: &str) -> std::io::Result<Vec<u16>> {
    let invalid = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid packaged application user model ID",
        )
    };
    if id.is_empty()
        || id.contains('\0')
        || id.len() > APPLICATION_USER_MODEL_ID_MAX_LENGTH as usize
    {
        return Err(invalid());
    }
    let id = id.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut family_length = 0;
    let mut application_length = 0;
    let result = unsafe {
        ParseApplicationUserModelId(
            PCWSTR(id.as_ptr()),
            &mut family_length,
            None,
            &mut application_length,
            None,
        )
    };
    if result != ERROR_INSUFFICIENT_BUFFER {
        return Err(invalid());
    }
    Ok(id)
}
