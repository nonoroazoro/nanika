use crate::ApplicationArguments;

#[test]
fn missing_windows_arguments_use_the_structured_empty_representation() {
    let direct = ApplicationArguments::empty();
    let shortcut = ApplicationArguments::from_windows_raw(None);

    assert_eq!(shortcut, direct);
    assert_eq!(
        shortcut.to_json().expect("arguments should encode"),
        r#"{"kind":"structured","values":[]}"#
    );
}

#[test]
fn windows_raw_arguments_have_an_explicit_tagged_representation() {
    let arguments = ApplicationArguments::from_windows_raw(Some("--profile work".to_owned()));

    assert_eq!(
        arguments.to_json().expect("arguments should encode"),
        r#"{"kind":"windowsRaw","value":"--profile work"}"#
    );
}
