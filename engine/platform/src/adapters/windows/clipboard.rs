use crate::PreparedClipboardContent;
use clipboard_win::{Clipboard, formats};

pub(crate) fn write(content: PreparedClipboardContent) -> Result<u64, String> {
    // Immediate-render formats outlive this message-only owner. No foreground
    // window, message polling, delayed renderer or retry policy is needed.
    let owner = super::ClipboardOwner::new()?;
    let clipboard =
        Clipboard::new_for(owner.handle()).map_err(|e| format!("OpenClipboard: {e}"))?;
    match content {
        PreparedClipboardContent::Text(value) => clipboard_win::set(formats::Unicode, value),
        PreparedClipboardContent::Files(paths) => {
            clipboard_win::raw::set_file_list_with(&paths, clipboard_win::options::DoClear)
        }
        PreparedClipboardContent::Png(bytes) => {
            let format =
                clipboard_win::register_format("PNG").ok_or("RegisterClipboardFormat: PNG")?;
            clipboard_win::set(formats::RawData(format.get()), bytes)
        }
    }
    .map_err(|e| format!("clipboard write failed: {e}"))?;
    // CloseClipboard finalizes synthesized text formats and advances the sequence.
    // Reacquire the native lock to bind the finalized sequence to our owner. This
    // is a receipt phase, not a write retry; contention or lost ownership fails.
    drop(clipboard);
    let _receipt = Clipboard::new_for(owner.handle())
        .map_err(|e| format!("clipboard receipt could not acquire ownership lock: {e}"))?;
    let current_owner = unsafe { windows::Win32::System::DataExchange::GetClipboardOwner() }
        .map_err(|e| format!("clipboard receipt owner is unavailable: {e}"))?;
    if current_owner.0 != owner.handle().cast() {
        return Err("Clipboard ownership changed before the write receipt.".into());
    }
    Ok(super::clipboard_revision())
}
