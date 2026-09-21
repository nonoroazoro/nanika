use std::io::Read as _;

use super::export_diagnostics;

#[test]
fn diagnostics_export_contains_metadata_and_logs() {
    let root = std::env::temp_dir().join(format!(
        "nanika-cli-diagnostics-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock must be after Unix epoch")
            .as_nanos()
    ));
    let logs = root.join("logs");
    std::fs::create_dir_all(&logs).expect("log directory should be created");
    std::fs::write(logs.join("nanika.2026-08-23.log"), "host starting\n")
        .expect("fixture log should be written");
    std::fs::write(logs.join("private.txt"), "must not be exported\n")
        .expect("stray file should be written");
    let destination = root.join("diagnostics.zip");

    export_diagnostics(&root, &destination).expect("diagnostics should export");

    let file = std::fs::File::open(&destination).expect("archive should exist");
    let mut archive = zip::ZipArchive::new(file).expect("archive should open");
    let mut metadata = String::new();
    archive
        .by_name("diagnostics.txt")
        .expect("metadata should exist")
        .read_to_string(&mut metadata)
        .expect("metadata should be readable");
    assert!(metadata.contains("Nanika version:"));
    assert!(archive.by_name("logs/nanika.2026-08-23.log").is_ok());
    assert!(archive.by_name("logs/private.txt").is_err());

    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

#[test]
fn diagnostics_never_overwrites_the_destination() {
    let root = temporary_root("no-clobber");
    std::fs::create_dir_all(root.join("logs")).expect("log directory should exist");
    let destination = root.join("diagnostics.zip");
    std::fs::write(&destination, "keep").expect("destination fixture should exist");

    assert!(export_diagnostics(&root, &destination).is_err());
    assert_eq!(
        std::fs::read_to_string(&destination).expect("destination should remain readable"),
        "keep"
    );
    std::fs::remove_dir_all(root).expect("test directory should be removed");
}

fn temporary_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "nanika-cli-diagnostics-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock must be after Unix epoch")
            .as_nanos()
    ))
}
