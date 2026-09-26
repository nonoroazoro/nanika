use std::path::Path;

use nanika_search::UsageKey;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult, params};

use crate::{
    ExtensionKind, StoredExtension, StoredExtensionLoad, StoredUsage,
    extension_id::is_valid_extension_id,
};

const SCHEMA: &str = "
CREATE TABLE extensions (
    extension_id TEXT PRIMARY KEY CHECK (extension_id <> ''),
    kind TEXT NOT NULL,
    version TEXT,
    install_path TEXT,
    package_digest TEXT,
    CHECK (
        (kind = 'built-in' AND version IS NULL AND install_path IS NULL AND package_digest IS NULL)
        OR (kind = 'external' AND version IS NOT NULL AND version <> '' AND install_path IS NOT NULL AND install_path <> '' AND package_digest IS NOT NULL AND package_digest <> '')
    )
) STRICT;
CREATE TABLE input_history (
    id INTEGER PRIMARY KEY,
    normalized_query TEXT NOT NULL UNIQUE CHECK (normalized_query <> ''),
    display_query TEXT NOT NULL CHECK (display_query <> ''),
    last_used_at INTEGER NOT NULL CHECK (last_used_at >= 0)
) STRICT;
CREATE INDEX input_history_last_used
ON input_history(last_used_at DESC, id DESC);
CREATE TABLE usage_stats (
    extension_id TEXT NOT NULL CHECK (extension_id <> ''),
    entry_id TEXT NOT NULL CHECK (entry_id <> ''),
    action_id TEXT NOT NULL CHECK (action_id <> ''),
    query_context TEXT NOT NULL,
    execution_count INTEGER NOT NULL CHECK (execution_count BETWEEN 1 AND 4294967295),
    last_executed_at INTEGER NOT NULL CHECK (last_executed_at >= 0),
    PRIMARY KEY (extension_id, entry_id, action_id, query_context),
    FOREIGN KEY (extension_id) REFERENCES extensions(extension_id) ON DELETE CASCADE
) STRICT, WITHOUT ROWID;
PRAGMA user_version=1;
";

pub struct HostDatabase {
    connection: Connection,
}

impl HostDatabase {
    pub fn open(path: impl AsRef<Path>) -> SqlResult<Self> {
        let connection = nanika_database::open(path, SCHEMA)?;
        Ok(Self { connection })
    }

    pub fn load_input_history(&self) -> SqlResult<Vec<String>> {
        let mut statement = self.connection.prepare(
            "SELECT display_query FROM input_history ORDER BY last_used_at DESC, id DESC",
        )?;
        let mut entries = statement
            .query_map([], |row| row.get(0))?
            .collect::<SqlResult<Vec<String>>>()?;
        entries.reverse();
        Ok(entries)
    }

    pub fn load_usage(&self) -> SqlResult<Vec<StoredUsage>> {
        let mut statement = self.connection.prepare(
            "SELECT extension_id, entry_id, action_id, query_context, execution_count, last_executed_at
             FROM usage_stats",
        )?;
        statement
            .query_map([], |row| {
                Ok(StoredUsage {
                    extension_id: row.get(0)?,
                    entry_id: row.get(1)?,
                    action_id: row.get(2)?,
                    query_context: row.get(3)?,
                    execution_count: row_u32(row, 4)?,
                    last_executed_at: row_u64(row, 5)?,
                })
            })?
            .collect()
    }

    pub fn load_extensions(&self) -> SqlResult<Vec<StoredExtension>> {
        Ok(self.load_extensions_isolated()?.extensions)
    }

    pub fn load_extensions_isolated(&self) -> SqlResult<StoredExtensionLoad> {
        let mut statement = self.connection.prepare(
            "SELECT extension_id, kind, version, install_path, package_digest
             FROM extensions
             ORDER BY extension_id",
        )?;
        let mut rows = statement.query([])?;
        let mut extensions = Vec::new();
        let mut errors = Vec::new();
        let mut row_number = 0_usize;
        while let Some(row) = rows.next()? {
            row_number += 1;
            let identity = row
                .get_ref(0)
                .ok()
                .and_then(|value| value.as_str().ok())
                .map(str::to_owned)
                .unwrap_or_else(|| format!("row {row_number}"));
            match stored_extension_from_row(row) {
                Ok(extension) => match validate_stored_extension_metadata(&extension) {
                    Ok(()) => extensions.push(extension),
                    Err(error) => errors.push(format!(
                        "extension {identity} has invalid metadata ({error}) and was skipped"
                    )),
                },
                Err(error) => errors.push(format!(
                    "extension {identity} has invalid metadata ({error}) and was skipped"
                )),
            }
        }
        Ok(StoredExtensionLoad { extensions, errors })
    }

    pub fn extension(&self, extension_id: &str) -> SqlResult<Option<StoredExtension>> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        self.connection
            .query_row(
                "SELECT extension_id, kind, version, install_path, package_digest
                 FROM extensions WHERE extension_id = ?1",
                params![extension_id],
                stored_extension_from_row,
            )
            .optional()
    }

    pub fn install_external_extension(
        &self,
        extension_id: &str,
        version: &str,
        install_path: &Path,
        package_digest: &str,
    ) -> SqlResult<()> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        self.connection.execute(
            "INSERT INTO extensions (
                extension_id, kind, version, install_path, package_digest
             ) VALUES (?1, 'external', ?2, ?3, ?4)
             ON CONFLICT(extension_id) DO UPDATE SET
                version = excluded.version,
                install_path = excluded.install_path,
                package_digest = excluded.package_digest
             WHERE extensions.kind = 'external'",
            params![
                extension_id,
                version,
                install_path.to_string_lossy(),
                package_digest,
            ],
        )?;
        Ok(())
    }

    pub fn remove_external_extension(&self, extension_id: &str) -> SqlResult<bool> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        Ok(self.connection.execute(
            "DELETE FROM extensions WHERE extension_id = ?1 AND kind = 'external'",
            params![extension_id],
        )? == 1)
    }

    pub(crate) fn record_input_history(
        &self,
        history_key: &str,
        display_query: &str,
        used_at: u64,
    ) -> SqlResult<()> {
        self.connection.execute(
            "INSERT INTO input_history (
                normalized_query, display_query, last_used_at
             ) VALUES (?1, ?2, ?3)
             ON CONFLICT(normalized_query) DO UPDATE SET
                display_query = excluded.display_query,
                last_used_at = excluded.last_used_at",
            params![
                history_key,
                display_query,
                i64::try_from(used_at).unwrap_or(i64::MAX),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn record_usage(
        &self,
        extension_id: &str,
        entry_id: &str,
        action_id: &str,
        query_context: &str,
        executed_at: u64,
    ) -> SqlResult<()> {
        self.connection.execute(
            "INSERT INTO usage_stats (
                extension_id, entry_id, action_id, query_context, execution_count, last_executed_at
             ) VALUES (?1, ?2, ?3, ?4, 1, ?5)
             ON CONFLICT(extension_id, entry_id, action_id, query_context) DO UPDATE SET
                execution_count = min(usage_stats.execution_count + 1, 4294967295),
                last_executed_at = excluded.last_executed_at",
            params![
                extension_id,
                entry_id,
                action_id,
                query_context,
                i64::try_from(executed_at).unwrap_or(i64::MAX),
            ],
        )?;
        Ok(())
    }

    pub(crate) fn record_execution(
        &self,
        history_key: &str,
        display_query: &str,
        usage: &UsageKey,
        history_used_at: u64,
        executed_at: u64,
    ) -> SqlResult<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO input_history (
                normalized_query, display_query, last_used_at
             ) VALUES (?1, ?2, ?3)
             ON CONFLICT(normalized_query) DO UPDATE SET
                display_query = excluded.display_query,
                last_used_at = excluded.last_used_at",
            params![
                history_key,
                display_query,
                i64::try_from(history_used_at).unwrap_or(i64::MAX),
            ],
        )?;
        transaction.execute(
            "INSERT INTO usage_stats (
                extension_id, entry_id, action_id, query_context, execution_count, last_executed_at
             ) VALUES (?1, ?2, ?3, ?4, 1, ?5)
             ON CONFLICT(extension_id, entry_id, action_id, query_context) DO UPDATE SET
                execution_count = min(usage_stats.execution_count + 1, 4294967295),
                last_executed_at = excluded.last_executed_at",
            params![
                usage.extension_id,
                usage.entry_id,
                usage.action_id,
                usage.query_context,
                i64::try_from(executed_at).unwrap_or(i64::MAX),
            ],
        )?;
        transaction.commit()
    }

    pub(crate) fn register_builtin_extension(&self, extension_id: &str) -> SqlResult<()> {
        if !is_valid_extension_id(extension_id) {
            return Err(rusqlite::Error::InvalidParameterName(
                "invalid extension id".to_owned(),
            ));
        }
        self.connection.execute(
            "INSERT INTO extensions (
                extension_id, kind
             ) VALUES (?1, 'built-in')
             ON CONFLICT(extension_id) DO UPDATE SET
                kind = excluded.kind,
                version = NULL,
                install_path = NULL,
                package_digest = NULL
             WHERE extensions.kind <> 'built-in'",
            params![extension_id],
        )?;
        Ok(())
    }

    pub(crate) fn reset_usage(&self) -> SqlResult<()> {
        self.connection.execute("DELETE FROM usage_stats", [])?;
        Ok(())
    }
}

fn stored_extension_from_row(row: &rusqlite::Row<'_>) -> SqlResult<StoredExtension> {
    let kind = row.get::<_, String>(1)?;
    let kind = ExtensionKind::parse(&kind).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(1, "kind".to_owned(), rusqlite::types::Type::Text)
    })?;
    Ok(StoredExtension {
        extension_id: row.get(0)?,
        kind,
        version: row.get(2)?,
        install_path: row.get::<_, Option<String>>(3)?.map(Into::into),
        package_digest: row.get(4)?,
    })
}

fn row_u32(row: &rusqlite::Row<'_>, index: usize) -> SqlResult<u32> {
    let value = row.get::<_, i64>(index)?;
    u32::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn row_u64(row: &rusqlite::Row<'_>, index: usize) -> SqlResult<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn validate_stored_extension_metadata(extension: &StoredExtension) -> Result<(), &'static str> {
    if !is_valid_extension_id(&extension.extension_id) {
        return Err("invalid extension id");
    }
    let package_fields = [
        extension.version.is_some(),
        extension.install_path.is_some(),
        extension.package_digest.is_some(),
    ];
    if extension.kind == ExtensionKind::External && package_fields.contains(&false) {
        return Err("external extension package metadata is incomplete");
    }
    if extension.kind == ExtensionKind::BuiltIn && package_fields.contains(&true) {
        return Err("built-in extension contains external package metadata");
    }
    if extension
        .version
        .as_deref()
        .is_some_and(|value| value.is_empty())
        || extension
            .install_path
            .as_deref()
            .is_some_and(|value| value.as_os_str().is_empty())
        || extension
            .package_digest
            .as_deref()
            .is_some_and(|value| value.is_empty())
    {
        return Err("external extension package metadata is empty");
    }
    Ok(())
}
