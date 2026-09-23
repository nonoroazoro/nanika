use super::*;

#[test]
fn metadata_preserves_launch_variants_and_rejects_ambiguous_or_oversized_input() {
    let root = std::env::temp_dir().join(format!("nanika-shim-metadata-{}", std::process::id()));
    let shims = root.join("shims");
    std::fs::create_dir_all(&shims).unwrap();
    let executable = shims.join("alias.exe");
    let metadata = executable.with_extension("shim");
    let target = std::env::current_exe().unwrap();
    std::fs::write(
        &metadata,
        format!(
            "\u{feff}# metadata\npath = \"{}\"\nargs = --profile \"Work Space\"\n",
            target.display()
        ),
    )
    .unwrap();
    let (resolved, arguments) = identity(&executable, Some("--new-window".into()))
        .unwrap()
        .unwrap();
    assert_eq!(resolved, target.canonicalize().unwrap());
    assert_eq!(
        arguments,
        ApplicationArguments::from_windows_raw(Some(
            "--profile \"Work Space\" --new-window".into()
        ))
    );

    for field in [
        "cwd = C:\\",
        "ENV = value",
        "elevate = true",
        "args = %OPTIONS%",
    ] {
        std::fs::write(
            &metadata,
            format!("path = \"{}\"\n{field}\n", target.display()),
        )
        .unwrap();
        assert_eq!(identity(&executable, None).unwrap().unwrap().0, executable);
    }
    for text in [
        "args = x".to_owned(),
        "path = a\npath = b".to_owned(),
        "x".repeat(MAX_SHIM_BYTES as usize + 1),
    ] {
        std::fs::write(&metadata, text).unwrap();
        assert!(identity(&executable, None).is_err());
    }
    std::fs::write(
        &metadata,
        format!("path = \"{}\"", root.join("missing.exe").display()),
    )
    .unwrap();
    assert!(identity(&executable, None).unwrap().is_none());
    std::fs::remove_file(&metadata).unwrap();
    assert_eq!(identity(&executable, None).unwrap().unwrap().0, executable);
    std::fs::remove_dir_all(root).unwrap();
}
