use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use nanika_extension_script::{ScriptConfig, discover_scripts};
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
    let entries = discover_scripts(&config).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.values().all(|entry| entry.title == "Build project"));
    assert_eq!(entries, discover_scripts(&config).unwrap());
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
    let entry = discover_scripts(&fixture.config())
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
    assert!(discover_scripts(&config).unwrap_err().contains("missing"));
    assert!(
        discover_scripts(&ScriptConfig { roots: Vec::new() })
            .unwrap()
            .is_empty()
    );
}
