use nucleo_matcher::Utf32Str;

/// An owned, normalized search value with reusable matcher characters.
/// ASCII borrows the existing bytes; Unicode storage follows the value's lifetime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextValue {
    _normalized: String,
    _unicode: Option<Box<[char]>>
}

impl TextValue {
    /// Prepare already-normalized text using the matcher's own grapheme semantics.
    pub fn new(normalized: String) -> Self {
        let mut characters = Vec::new();
        let unicode = matches!(
            Utf32Str::new(&normalized, &mut characters),
            Utf32Str::Unicode(_)
        )
        .then(|| characters.into_boxed_slice());
        Self {
            _normalized: normalized,
            _unicode: unicode
        }
    }

    pub(crate) fn normalized(&self) -> &str {
        &self._normalized
    }

    pub(crate) fn into_normalized(self) -> String {
        self._normalized
    }

    pub(crate) fn characters(&self) -> Utf32Str<'_> {
        match &self._unicode {
            Some(characters) => Utf32Str::Unicode(characters),
            None => Utf32Str::Ascii(self._normalized.as_bytes())
        }
    }
}
