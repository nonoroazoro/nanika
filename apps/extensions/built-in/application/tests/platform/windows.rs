use super::*;

#[test]
fn packaged_inspection_failure_retains_the_resolved_path_for_cleanup() {
    let root = std::env::temp_dir().join(format!("nanika-packaged-root-{}", std::process::id()));
    std::fs::write(&root, "not a directory").unwrap();
    let mut roots = DiscoveryRoots::default();
    _include_packaged_root(&mut roots, SYSTEM_PACKAGED_KEY, Ok(root.clone()));
    assert!(roots.paths.is_empty());
    assert_eq!(roots.failures.len(), 1);
    assert_eq!(roots.failures[0].path.as_ref(), Some(&root));
    let mut coverage = crate::scan_coverage::ScanCoverage::new(
        std::iter::empty(),
        roots.failures.iter().all(|failure| failure.path.is_some()),
    );
    coverage.failed(roots.failures[0].path.as_deref().unwrap());
    assert!(!coverage.replaces(&path_key(&root.join("Existing.exe"))));
    assert!(coverage.replaces(&path_key(&root.with_extension("removed").join("Old.exe"))));
    std::fs::remove_file(root).unwrap();
}

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
            "--nocapture",
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
