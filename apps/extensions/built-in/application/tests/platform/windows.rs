use super::*;

#[test]
fn scoop_uses_only_the_selected_installations_shims() {
    let root = known_folder(&FOLDERID_Profile)
        .unwrap()
        .join("custom scoop");
    assert_eq!(
        scoop_shim_root(Some(root.clone().into_os_string()), &FOLDERID_ProgramData).unwrap(),
        root.join("shims")
    );
    assert_eq!(
        scoop_shim_root(None, &FOLDERID_Profile).unwrap(),
        known_folder(&FOLDERID_Profile).unwrap().join("scoop/shims")
    );
    assert_eq!(
        scoop_shim_root(None, &FOLDERID_ProgramData).unwrap(),
        known_folder(&FOLDERID_ProgramData)
            .unwrap()
            .join("scoop/shims")
    );
}

#[test]
fn invalid_scoop_override_is_an_error_instead_of_a_disabled_source() {
    for root in ["", "relative/scoop"] {
        assert!(scoop_shim_root(Some(root.into()), &FOLDERID_Profile).is_err());
    }
}

#[test]
fn disabling_all_builtin_sources_produces_no_roots() {
    assert!(standard_roots(|_| false).unwrap().paths.is_empty());
}

#[test]
fn a_failed_source_does_not_block_other_builtin_sources() {
    const PROBE: &str = "NANIKA_TEST_SOURCE_FAILURE";
    if std::env::var_os(PROBE).is_some() {
        let roots =
            standard_roots(|key| key == SCOOP_USER_KEY || key == SYSTEM_PROGRAMS_KEY).unwrap();
        assert_eq!(
            roots.paths,
            vec![known_folder(&FOLDERID_CommonPrograms).unwrap()]
        );
        assert_eq!(roots.failures.len(), 1);
        assert!(roots.failures[0].message.contains(SCOOP_USER_KEY));
        assert!(roots.failures[0].message.contains("must be absolute"));
        assert!(roots.failures[0].path.is_none());
        return;
    }
    // Isolate environment overrides from concurrently running tests.
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "platform::windows::tests::a_failed_source_does_not_block_other_builtin_sources",
            "--nocapture"
        ])
        .env(PROBE, "1")
        .env(SCOOP_ENVIRONMENT, "relative/scoop")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
}
