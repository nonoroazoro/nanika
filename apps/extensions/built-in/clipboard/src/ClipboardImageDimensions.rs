/// Facts captured from the image before it enters history, never inferred from a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardImageDimensions {
    pub width: u32,
    pub height: u32
}
