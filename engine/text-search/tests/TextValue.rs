use crate::TextValue;
use nucleo_matcher::Utf32Str;

#[test]
fn ascii_reuses_the_owned_text_bytes() {
    let value = TextValue::new("application 42".into());
    let Utf32Str::Ascii(bytes) = value.characters() else {
        panic!("ASCII must not allocate a second character representation");
    };
    assert_eq!(bytes.as_ptr(), value.normalized().as_ptr());
}

#[test]
fn unicode_preserves_matcher_grapheme_semantics_and_original_text() {
    for (raw, expected) in [
        ("应用程序", vec!['应', '用', '程', '序']),
        ("résumé", vec!['r', 'é', 's', 'u', 'm', 'é']),
        ("👨‍👩‍👧‍👦", vec!['👨']),
        ("東京", vec!['東', '京'])
    ] {
        let value = TextValue::new(raw.into());
        assert_eq!(value.characters(), Utf32Str::Unicode(&expected));
        assert_eq!(value.normalized(), raw);
        let retained = value.clone();
        drop(value);
        assert_eq!(retained.characters(), Utf32Str::Unicode(&expected));
        assert_eq!(retained.into_normalized(), raw);
    }
}
