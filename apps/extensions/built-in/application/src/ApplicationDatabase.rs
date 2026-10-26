use crate::ApplicationEntryData;
use crate::{ApplicationEntry, ApplicationError};
use rusqlite::{Connection, params};
use std::collections::HashMap;
use std::path::Path;

pub(crate) const SCHEMA: &str = "
CREATE TABLE app_sources (
    source_id TEXT NOT NULL CHECK (source_id <> ''),
    entry_id TEXT NOT NULL CHECK (entry_id <> ''),
    source_key TEXT NOT NULL CHECK (source_key <> ''),
    display_name TEXT NOT NULL CHECK (display_name <> ''),
    normalized_tokens TEXT NOT NULL,
    launch_kind TEXT NOT NULL CHECK (launch_kind IN ('macos-bundle', 'windows-shell-link', 'windows-packaged', 'executable')),
    target_path TEXT NOT NULL CHECK (target_path <> ''),
    arguments_json TEXT NOT NULL CHECK (arguments_json <> ''),
    icon_key TEXT NOT NULL,
    icon_source TEXT,
    priority INTEGER NOT NULL CHECK (priority >= 0),
    localized_names TEXT NOT NULL,
    PRIMARY KEY (source_id, entry_id)
) STRICT;
PRAGMA user_version=1;
";

/// Durable source-owned records. Winner selection belongs to the extension.
pub struct ApplicationDatabase {
    connection: Connection
}

impl ApplicationDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ApplicationError> {
        Ok(Self {
            connection: crate::migration::open(path.as_ref())?
        })
    }

    pub(crate) fn load_sources(
        &self
    ) -> Result<HashMap<String, HashMap<String, ApplicationEntry>>, ApplicationError> {
        let mut statement = self.connection.prepare("SELECT source_id, entry_id, source_key, display_name, normalized_tokens, launch_kind, target_path, arguments_json, icon_key, icon_source, priority, localized_names FROM app_sources")?;
        let rows = statement.query_map([], |row| {
            let display_name: String = row.get(3)?;
            let entry = ApplicationEntry::new(ApplicationEntryData {
                localized_names:
                    serde_json::from_str::<std::collections::BTreeMap<String, String>>(
                        &row.get::<_, String>(11)?
                    )
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            11,
                            rusqlite::types::Type::Text,
                            Box::new(error)
                        )
                    })?,
                entry_id: row.get(1)?,
                source_key: row.get(2)?,
                normalized_name: crate::normalization::normalize_name(&display_name),
                display_name,
                normalized_tokens: row.get(4)?,
                launch_kind: row.get(5)?,
                target_path: row.get(6)?,
                arguments_json: row.get(7)?,
                icon_key: row.get(8)?,
                icon_source: row
                    .get::<_, Option<String>>(9)?
                    .map(|json| {
                        serde_json::from_str(&json).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                9,
                                rusqlite::types::Type::Text,
                                Box::new(error)
                            )
                        })
                    })
                    .transpose()?,
                priority: row.get::<_, u32>(10)? as usize
            });
            Ok((row.get::<_, String>(0)?, entry))
        })?;
        let mut roots = HashMap::<String, HashMap<String, ApplicationEntry>>::new();
        for row in rows {
            let (root, entry) = row?;
            roots
                .entry(root)
                .or_default()
                .insert(entry.entry_id.clone(), entry);
        }
        Ok(roots)
    }

    pub fn load_entries(&self) -> Result<Vec<ApplicationEntry>, ApplicationError> {
        let mut sources = crate::application_sources::ApplicationSources::default();
        for (root, entries) in self.load_sources()? {
            sources.commit(root, entries);
        }
        Ok(sources.winners())
    }

    /// Persist only changed source rows after discovery, including non-winning alternatives.
    pub fn commit_source(
        &mut self,
        root: &str,
        entries: &[ApplicationEntry],
        removed: &[String]
    ) -> Result<(), ApplicationError> {
        if entries.is_empty() && removed.is_empty() {
            return Ok(());
        }
        let transaction = self.connection.transaction()?;
        {
            let mut remove = transaction
                .prepare("DELETE FROM app_sources WHERE source_id = ?1 AND entry_id = ?2")?;
            for id in removed {
                remove.execute(params![root, id])?;
            }
            let mut upsert = transaction.prepare("INSERT INTO app_sources (source_id, entry_id, source_key, display_name, normalized_tokens, launch_kind, target_path, arguments_json, icon_key, icon_source, priority, localized_names) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) ON CONFLICT(source_id, entry_id) DO UPDATE SET source_key=excluded.source_key, display_name=excluded.display_name, normalized_tokens=excluded.normalized_tokens, launch_kind=excluded.launch_kind, target_path=excluded.target_path, arguments_json=excluded.arguments_json, icon_key=excluded.icon_key, icon_source=excluded.icon_source, priority=excluded.priority, localized_names=excluded.localized_names")?;
            for entry in entries {
                upsert.execute(params![
                    root,
                    entry.entry_id,
                    entry.source_key,
                    entry.display_name,
                    entry.normalized_tokens,
                    entry.launch_kind,
                    entry.target_path,
                    entry.arguments_json,
                    entry.icon_key,
                    entry
                        .icon_source
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()?,
                    i64::try_from(entry.priority).map_err(|error| {
                        rusqlite::Error::ToSqlConversionFailure(Box::new(error))
                    })?,
                    serde_json::to_string(&entry.localized_names)?
                ])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }
}
