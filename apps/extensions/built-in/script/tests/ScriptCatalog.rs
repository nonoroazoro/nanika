use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nanika_extension_script::{ScriptCatalog, ScriptConfig};
use nanika_protocol::{LaunchArguments, LaunchDescriptor};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "nanika-script-catalog-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn config(&self) -> ScriptConfig {
        ScriptConfig {
            roots: vec![self.0.clone()],
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn discovers_nested_scripts_deduplicates_roots_and_ignores_other_files() {
    let fixture = Fixture::new();
    let nested = fixture.0.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    for path in [
        fixture.0.join("Build project.py"),
        nested.join("Build project.PS1"),
    ] {
        std::fs::write(path, b"this file must never execute during discovery").unwrap();
    }
    std::fs::write(fixture.0.join("README.md"), b"documentation").unwrap();
    let mut config = fixture.config();
    config.roots.push(nested);
    let entries = discover(&config).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.values().all(|entry| entry.title == "Build project"));
    assert_eq!(entries, discover(&config).unwrap());
    assert!(
        entries
            .values()
            .all(|entry| entry.candidate().entry_id == entry.id)
    );
}

#[test]
fn launch_keeps_paths_with_spaces_and_metacharacters_in_one_argument() {
    let fixture = Fixture::new();
    let script = fixture.0.join("build & report.py");
    std::fs::write(&script, b"print(1)").unwrap();
    let entry = discover(&fixture.config())
        .unwrap()
        .into_values()
        .next()
        .unwrap();
    let LaunchDescriptor::Program {
        arguments: LaunchArguments::Structured { values },
        working_directory,
        ..
    } = entry.launch_descriptor().unwrap()
    else {
        panic!("expected structured program launch");
    };
    assert_eq!(
        values,
        vec![script.canonicalize().unwrap().to_str().unwrap()]
    );
    assert_eq!(
        working_directory.as_deref(),
        fixture.0.canonicalize().unwrap().to_str()
    );
    std::fs::remove_file(script).unwrap();
    assert!(
        entry
            .launch_descriptor()
            .unwrap_err()
            .contains("unavailable")
    );
}

#[test]
fn failed_root_is_an_error_and_empty_configuration_is_an_empty_catalog() {
    let fixture = Fixture::new();
    let config = ScriptConfig {
        roots: vec![fixture.0.join("missing")],
    };
    assert!(discover(&config).unwrap_err().contains("missing"));
    assert!(
        discover(&ScriptConfig { roots: Vec::new() })
            .unwrap()
            .is_empty()
    );
}

fn discover(
    config: &ScriptConfig,
) -> Result<std::collections::BTreeMap<String, nanika_extension_script::ScriptEntry>, String> {
    let mut catalog = ScriptCatalog::default();
    catalog.scan(config, || false, |_, _| {})?;
    Ok(catalog.entries())
}

#[test]
fn cancellation_preserves_unvisited_roots_and_failure_does_not_block_other_roots() {
    use std::sync::atomic::AtomicBool;
    let fixture = Fixture::new();
    let first = fixture.0.join("a");
    let second = fixture.0.join("b");
    std::fs::create_dir_all(first.join("nested")).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("nested/old-a.py"), "").unwrap();
    std::fs::write(second.join("old-b.py"), "").unwrap();
    let config = ScriptConfig {
        roots: vec![first.clone(), second.clone()],
    };
    let mut catalog = ScriptCatalog::default();
    catalog.scan(&config, || false, |_, _| {}).unwrap();
    std::fs::remove_file(first.join("nested/old-a.py")).unwrap();
    std::fs::remove_file(second.join("old-b.py")).unwrap();
    std::fs::write(first.join("nested/new-a.py"), "").unwrap();
    std::fs::write(second.join("new-b.py"), "").unwrap();
    let cancelled = AtomicBool::new(false);
    assert!(
        catalog
            .scan(
                &config,
                || cancelled.load(Ordering::Acquire),
                |entries, _| {
                    assert!(entries.iter().any(|entry| entry.title == "new-a"));

                    assert!(!entries.iter().any(|entry| entry.title == "new-b"));
                    cancelled.store(true, Ordering::Release);
                }
            )
            .is_err()
    );
    std::fs::remove_dir_all(&first).unwrap();
    assert!(catalog.scan(&config, || false, |_, _| {}).is_err());
    let entries = catalog.entries();
    assert!(entries.values().any(|entry| entry.title == "new-a"));
    assert!(entries.values().any(|entry| entry.title == "new-b"));
    assert!(!entries.values().any(|entry| entry.title == "old-b"));
}

#[test]
fn unchanged_script_catalog_does_not_publish_and_root_deletion_is_local() {
    let fixture = Fixture::new();
    let first = fixture.0.join("a");
    let second = fixture.0.join("b");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("a.py"), "").unwrap();
    std::fs::write(second.join("b.py"), "").unwrap();
    let mut config = ScriptConfig {
        roots: vec![first, second],
    };
    let mut catalog = ScriptCatalog::default();
    catalog.scan(&config, || false, |_, _| {}).unwrap();
    catalog
        .scan(
            &config,
            || false,
            |_, _| panic!("unchanged roots must not publish"),
        )
        .unwrap();
    config.roots.pop();
    let mut patches = 0;
    catalog
        .scan(
            &config,
            || false,
            |updated, removed| {
                patches += 1;
                assert!(updated.is_empty());
                assert_eq!(removed.len(), 1);
            },
        )
        .unwrap();
    assert_eq!(patches, 1);
    assert_eq!(catalog.entries().len(), 1);
}

#[test]
fn large_roots_are_complete_and_can_be_replaced_by_a_different_directory() {
    let fixture = Fixture::new();
    let large = fixture.0.join("large");
    let small = fixture.0.join("small");
    std::fs::create_dir_all(&large).unwrap();
    std::fs::create_dir_all(&small).unwrap();
    for index in 0..5001 {
        std::fs::write(large.join(format!("script-{index}.py")), "").unwrap();
    }
    std::fs::write(small.join("replacement.py"), "").unwrap();
    let mut catalog = ScriptCatalog::default();
    let mut published = 0;
    catalog
        .scan(
            &ScriptConfig { roots: vec![large] },
            || false,
            |updated, removed| {
                published += updated.len();
                assert!(removed.is_empty());
            },
        )
        .unwrap();
    assert_eq!(published, 5001);
    assert_eq!(catalog.entries().len(), 5001);
    let mut removed_count = 0;
    catalog
        .scan(
            &ScriptConfig { roots: vec![small] },
            || false,
            |_, removed| {
                removed_count += removed.len();
            },
        )
        .unwrap();
    assert_eq!(removed_count, 5001);
    let entries = catalog.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries.values().next().unwrap().title, "replacement");
}
