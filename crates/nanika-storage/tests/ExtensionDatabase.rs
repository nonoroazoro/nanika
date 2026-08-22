use crate::{ExtensionDatabase, NanikaPaths};

#[test]
fn database_is_isolated_by_extension_id() {
    let root = std::env::temp_dir().join(format!("nanika-storage-ext-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let paths = NanikaPaths::from_roots(&root, root.join("cache"), root.join("config"));
    let first = ExtensionDatabase::open(&paths, "example.one").expect("extension db");
    let second = ExtensionDatabase::open(&paths, "example.two").expect("extension db");
    assert_eq!(first.schema_version().expect("schema version"), 0);
    assert_eq!(second.schema_version().expect("schema version"), 0);
    assert!(root.join("databases/extensions/example.one.db").is_file());
    assert!(!root.join("config/example.one.db").exists());
    assert!(ExtensionDatabase::open(&paths, "../escape").is_err());
    assert!(ExtensionDatabase::open(&paths, "con.extension").is_err());
    assert!(ExtensionDatabase::open(&paths, "Example.Extension").is_err());
    let _ = std::fs::remove_dir_all(root);
}
