//! Host-owned paths and SQLite storage.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use rusqlite::{Connection, Result as SqlResult, params};

const QUALIFIER: &str = "com";
const ORGANIZATION: &str = "nanika";
const APPLICATION: &str = "nanika";

/// Resolved machine-local and user-configurable Nanika locations.
#[derive(Debug, Clone)]
pub struct NanikaPaths {
    app_data_root: PathBuf,
    cache_root: PathBuf,
    config_root: PathBuf,
}

impl NanikaPaths {
    /// Resolve the platform-standard roots for the current user.
    pub fn discover() -> Option<Self> {
        ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).map(|dirs| Self {
            app_data_root: dirs.data_local_dir().to_path_buf(),
            cache_root: dirs.cache_dir().to_path_buf(),
            config_root: dirs.config_dir().to_path_buf(),
        })
    }

    /// Build paths from explicit roots, primarily for tests and relocation.
    pub fn from_roots(app_data_root: impl Into<PathBuf>, cache_root: impl Into<PathBuf>) -> Self {
        let app_data_root = app_data_root.into();
        Self {
            config_root: app_data_root.join("config"),
            cache_root: cache_root.into(),
            app_data_root,
        }
    }

    pub fn app_data_root(&self) -> &Path {
        &self.app_data_root
    }

    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    pub fn config_root(&self) -> &Path {
        &self.config_root
    }

    pub fn bootstrap_file(&self) -> PathBuf {
        self.app_data_root.join("bootstrap.jsonc")
    }

    pub fn database_dir(&self) -> PathBuf {
        self.app_data_root.join("databases")
    }

    pub fn host_database(&self) -> PathBuf {
        self.database_dir().join("nanika.db")
    }

    pub fn ensure_machine_local_dirs(&self) -> std::io::Result<()> {
        fs::create_dir_all(self.database_dir())?;
        fs::create_dir_all(self.app_data_root.join("extensions"))?;
        fs::create_dir_all(self.app_data_root.join("logs"))?;
        fs::create_dir_all(self.cache_root.join("icons"))?;
        fs::create_dir_all(self.cache_root.join("metadata"))
    }
}

const MIGRATIONS: &[(i64, &str)] = &[(
    1,
    "
CREATE TABLE extensions (
    extension_id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    installed_version TEXT,
    active_version TEXT,
    install_path TEXT,
    package_digest TEXT,
    state TEXT NOT NULL,
    health TEXT NOT NULL,
    last_error TEXT,
    updated_at INTEGER NOT NULL
);
CREATE TABLE input_history (
    id INTEGER PRIMARY KEY,
    normalized_query TEXT NOT NULL UNIQUE,
    display_query TEXT NOT NULL,
    use_count INTEGER NOT NULL DEFAULT 0,
    first_used_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL
);
CREATE TABLE usage_stats (
    extension_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    query_context TEXT NOT NULL,
    execution_count INTEGER NOT NULL DEFAULT 0,
    last_executed_at INTEGER NOT NULL,
    PRIMARY KEY (extension_id, action_id, query_context),
    FOREIGN KEY (extension_id) REFERENCES extensions(extension_id) ON DELETE CASCADE
);
",
)];

/// Host-owned SQLite database with embedded forward-only migrations.
pub struct HostDatabase {
    connection: Connection,
}

/// Extension-owned SQLite database with an isolated schema migration table.
pub struct ExtensionDatabase {
    connection: Connection,
}

impl HostDatabase {
    pub fn open(path: impl AsRef<Path>) -> SqlResult<Self> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=100;",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
        )?;

        let current: i64 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        for (version, sql) in MIGRATIONS
            .iter()
            .copied()
            .filter(|(version, _)| *version > current)
        {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, unixepoch())",
                params![version],
            )?;
            transaction.commit()?;
        }

        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> SqlResult<i64> {
        self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
    }
}

impl ExtensionDatabase {
    pub fn open(paths: &NanikaPaths, extension_id: &str) -> SqlResult<Self> {
        if extension_id.is_empty()
            || extension_id == "."
            || extension_id == ".."
            || extension_id.contains('/')
            || extension_id.contains('\\')
        {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        let directory = paths.database_dir().join("extensions");
        fs::create_dir_all(&directory)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let connection = Connection::open(directory.join(format!("{extension_id}.db")))?;
        connection.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=100;",
        )?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
        )?;
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> SqlResult<i64> {
        self.connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{HostDatabase, NanikaPaths};

    #[test]
    fn paths_keep_config_and_generated_data_separate() {
        let paths = NanikaPaths::from_roots("config", "cache");
        assert!(paths.host_database().ends_with("databases/nanika.db"));
        assert!(paths.bootstrap_file().ends_with("bootstrap.jsonc"));
    }

    #[test]
    fn host_database_applies_migrations_once() {
        let database =
            std::env::temp_dir().join(format!("nanika-storage-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&database);
        let _ = std::fs::remove_file(database.with_extension("db-wal"));
        let _ = std::fs::remove_file(database.with_extension("db-shm"));
        let first = HostDatabase::open(&database).expect("database should open");
        assert_eq!(first.schema_version().expect("schema version"), 1);
        drop(first);
        let second = HostDatabase::open(&database).expect("database should reopen");
        assert_eq!(second.schema_version().expect("schema version"), 1);
        drop(second);
        let _ = std::fs::remove_file(&database);
        let _ = std::fs::remove_file(database.with_extension("db-wal"));
        let _ = std::fs::remove_file(database.with_extension("db-shm"));
    }

    #[test]
    fn extension_database_isolated_by_extension_id() {
        let root = std::env::temp_dir().join(format!("nanika-storage-ext-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = NanikaPaths::from_roots(&root, root.join("cache"));
        let first = super::ExtensionDatabase::open(&paths, "example.one").expect("extension db");
        let second = super::ExtensionDatabase::open(&paths, "example.two").expect("extension db");
        assert_eq!(first.schema_version().expect("schema version"), 0);
        assert_eq!(second.schema_version().expect("schema version"), 0);
        assert!(root.join("databases/extensions/example.one.db").is_file());
        assert!(!root.join("config/example.one.db").exists());
        assert!(super::ExtensionDatabase::open(&paths, "../escape").is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
