use crate::{
    ClipboardChange, ClipboardConfig, ClipboardDatabase, ClipboardEntry, ClipboardPresentation,
    ClipboardQuery, ClipboardViewState
};
use std::{collections::HashMap, path::Path, sync::Arc};

/// The clipboard owner's durable data, continuation authority and incremental query cache. No shared raw history.
pub struct ClipboardStore {
    _syntax_highlighting: bool,
    _database: ClipboardDatabase,
    _query: Option<ClipboardQuery>,
    _query_revision: u64,
    _window: Option<crate::ClipboardWindow>,
    _preview: Option<Arc<crate::ClipboardPreview>>,
    _text_reader: crate::ClipboardTextReader,
    _text_revision: u64,
    _text_revisions: HashMap<String, u64>
}

impl ClipboardStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        Ok(Self {
            _syntax_highlighting: true,
            _database: ClipboardDatabase::open(path)?,
            _query: None,
            _query_revision: 0,
            _window: None,
            _preview: None,
            _text_reader: crate::ClipboardTextReader::default(),
            _text_revision: 0,
            _text_revisions: HashMap::new()
        })
    }

    pub(crate) fn retained_images(
        &self
    ) -> Result<std::collections::HashSet<std::path::PathBuf>, String> {
        self._database.retained_images()
    }

    pub fn capture(
        &mut self,
        entry: &ClipboardEntry,
        now: u64,
        config: &ClipboardConfig
    ) -> Result<ClipboardChange, String> {
        let change = self._database.upsert_with_retention(entry, now, config)?;
        self._changed(Some(entry), &change);
        Ok(change)
    }

    pub fn apply_retention(
        &mut self,
        now: u64,
        config: &ClipboardConfig
    ) -> Result<ClipboardChange, String> {
        let change = self._database.apply_retention(now, config)?;
        if !change.removed.is_empty() {
            self._changed(None, &change);
        }
        Ok(change)
    }

    pub(crate) fn all_ids(&self) -> Result<Vec<String>, String> {
        self._database.matching_ids("", "all")
    }

    pub fn clear(&mut self, ids: &[String]) -> Result<ClipboardChange, String> {
        let change = self._database.clear(ids)?;
        if !change.removed.is_empty() {
            self._changed(None, &change);
        }
        Ok(change)
    }

    /// The owning view releases transient query and payload caches at its close boundary.
    pub fn close_view(&mut self) {
        self._query = None;
        self._window = None;
        self._preview = None;
        self._text_reader.publish(None);
    }

    pub fn set_syntax_highlighting(&mut self, enabled: bool) {
        self._syntax_highlighting = enabled;
    }

    pub fn text_reader(&self) -> crate::ClipboardTextReader {
        self._text_reader.clone()
    }

    pub fn content(&self, id: &str) -> Result<nanika_protocol::ClipboardContent, String> {
        self._database.entry(id).map(|entry| entry.content)
    }

    pub fn present(
        &mut self,
        mut state: ClipboardViewState,
        expected_revision: Option<u64>
    ) -> Result<ClipboardPresentation, String> {
        if expected_revision.is_some_and(|revision| revision != self._query_revision) {
            return Err("Clipboard history changed. Use its current view.".into());
        }
        nanika_protocol::validate_view_search_text(&state.query)?;
        let query = state.query.trim().to_lowercase();
        if self._query.as_ref().is_none_or(|current| {
            current.text != query || current.content_type != state.content_type
        }) {
            let entries = self
                ._database
                .matching_entries(&query, &state.content_type)?;
            let ids = entries.iter().map(|entry| entry.entry_id.clone()).collect();
            let positions = entries
                .iter()
                .enumerate()
                .map(|(index, entry)| (entry.entry_id.clone(), index))
                .collect();
            self._query_revision += 1;
            self._query = Some(ClipboardQuery {
                text: query,
                content_type: state.content_type.clone(),
                entries,
                positions,
                entry_ids: Arc::new(ids)
            });
        }
        if state.count == 0 || state.count > nanika_protocol::MAX_VIEW_ITEMS {
            return Err("Clipboard viewport demand is outside the supported range.".into());
        }
        let query = self._query.as_ref().expect("current query");
        let ids = Arc::clone(&query.entry_ids);
        let collection_id = self._query_revision.to_string();
        if state.offset > 0
            && let Some(index) = state
                .anchor_id
                .as_ref()
                .and_then(|id| query.positions.get(id))
        {
            state.offset = *index;
        }
        if state.offset >= ids.len() {
            state.offset = ids.len().saturating_sub(state.count);
        }
        let end = state.offset.saturating_add(state.count).min(ids.len());
        let visible_ids = &ids[state.offset..end];
        state.anchor_id = visible_ids.first().cloned();
        if state
            .selected_item_id
            .as_ref()
            .is_none_or(|id| !query.positions.contains_key(id))
        {
            state.selected_item_id = visible_ids.first().cloned();
        }
        let selected_index = state
            .selected_item_id
            .as_ref()
            .and_then(|id| query.positions.get(id))
            .copied();
        if self._window.as_ref().is_none_or(|window| {
            window.collection_id != collection_id
                || window.offset != state.offset
                || window.count != state.count
        }) {
            self._window = Some(crate::ClipboardWindow {
                collection_id: collection_id.clone(),
                offset: state.offset,
                count: state.count,
                items: self._database.items(visible_ids)?
            });
        }
        if self
            ._preview
            .as_ref()
            .map(|preview| &preview.entry.entry_id)
            != state.selected_item_id.as_ref()
        {
            self._preview = state
                .selected_item_id
                .as_deref()
                .map(|id| {
                    self._database
                        .entry(id)
                        .and_then(|entry| {
                            let revision = self._text_revisions.get(id).copied().unwrap_or(0);
                            crate::ClipboardPreview::new(entry, revision.to_string())
                        })
                        .map(Arc::new)
                })
                .transpose()?;
        }
        self._text_reader.publish(self._preview.clone());
        let items = &self._window.as_ref().expect("current window").items;
        let selected = self._preview.as_deref();
        let mut paths = HashMap::new();
        for item in items {
            if let Some(path) = &item.first_path {
                paths.insert(item.entry_id.clone(), vec![path.clone()]);
            }
        }
        if let Some(ClipboardEntry {
            entry_id,
            content:
                nanika_protocol::ClipboardContent::Files {
                    paths: selected_paths
                },
            ..
        }) = selected.map(|preview| &preview.entry)
        {
            paths.insert(
                entry_id.clone(),
                selected_paths
                    .iter()
                    .take(crate::FILE_COLLECTION_PREVIEW_LIMIT)
                    .cloned()
                    .collect()
            );
        }
        let view = crate::view::clipboard_view(
            self._syntax_highlighting,
            &state,
            items,
            selected,
            selected_index,
            ids.len(),
            collection_id
        )?;
        view.validate()?;
        Ok(ClipboardPresentation {
            state,
            view,
            collection_revision: self._query_revision,
            matching_ids: ids,
            paths
        })
    }

    fn _changed(&mut self, upsert: Option<&ClipboardEntry>, change: &ClipboardChange) {
        // Revisions belong to committed mutations, not selection. The host's
        // instance scope prevents reuse across process restarts. Retain only
        // metadata for retained entries; deleting and reinserting an ID receives
        // a fresh revision without hashing its payload on the navigation path.
        if let Some(entry) = upsert {
            self._text_revision += 1;
            self._text_revisions
                .insert(entry.entry_id.clone(), self._text_revision);
        }
        for id in &change.removed {
            self._text_revisions.remove(id);
        }
        if self._preview.as_ref().is_some_and(|preview| {
            change.removed.contains(&preview.entry.entry_id)
                || upsert.is_some_and(|entry| entry.entry_id == preview.entry.entry_id)
        }) {
            self._preview = None;
            self._text_reader.publish(None);
        }
        if let Some(query) = &mut self._query
            && query.apply(upsert, &change.removed)
        {
            self._query_revision += 1;
        }
    }
}

#[cfg(test)]
#[path = "../tests/ClipboardStore.rs"]
mod tests;
