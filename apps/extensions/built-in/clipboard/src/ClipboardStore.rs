use crate::{
    CLIPBOARD_PAGE_SIZE, ClipboardChange, ClipboardConfig, ClipboardDatabase, ClipboardEntry,
    ClipboardPresentation, ClipboardQuery, ClipboardViewState,
};
use std::{collections::HashMap, path::Path, sync::Arc};

/// The clipboard owner's durable data, page authority and incremental query cache. No shared raw history.
pub struct ClipboardStore {
    _database: ClipboardDatabase,
    _revision: u64,
    _query: Option<ClipboardQuery>,
}

impl ClipboardStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        Ok(Self {
            _database: ClipboardDatabase::open(path)?,
            _revision: 0,
            _query: None,
        })
    }

    pub(crate) fn retained_images(
        &self,
    ) -> Result<std::collections::HashSet<std::path::PathBuf>, String> {
        self._database.retained_images()
    }

    pub fn capture(
        &mut self,
        entry: &ClipboardEntry,
        now: u64,
        config: &ClipboardConfig,
    ) -> Result<ClipboardChange, String> {
        let change = self._database.upsert_with_retention(entry, now, config)?;
        self._changed(Some(entry), &change);
        Ok(change)
    }

    pub fn apply_retention(
        &mut self,
        now: u64,
        config: &ClipboardConfig,
    ) -> Result<ClipboardChange, String> {
        let change = self._database.apply_retention(now, config)?;
        if !change.removed.is_empty() {
            self._changed(None, &change);
        }
        Ok(change)
    }

    pub fn clear(&mut self, ids: &[String]) -> Result<ClipboardChange, String> {
        let change = self._database.clear(ids)?;
        if !change.removed.is_empty() {
            self._changed(None, &change);
        }
        Ok(change)
    }

    pub fn content(&self, id: &str) -> Result<nanika_protocol::ClipboardContent, String> {
        self._database.entry(id).map(|entry| entry.content)
    }

    pub fn present(
        &mut self,
        mut state: ClipboardViewState,
        expected_revision: Option<u64>,
    ) -> Result<ClipboardPresentation, String> {
        if expected_revision.is_some_and(|revision| revision != self._revision) {
            return Err("Clipboard history changed. Use its current page controls.".into());
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
            self._query = Some(ClipboardQuery {
                text: query,
                content_type: state.content_type.clone(),
                entries,
                entry_ids: Arc::new(ids),
            });
        }
        let ids = Arc::clone(&self._query.as_ref().expect("current query").entry_ids);
        if let Some(index) = state
            .selected_item_id
            .as_ref()
            .and_then(|id| ids.iter().position(|candidate| candidate == id))
        {
            state.page_offset = index / CLIPBOARD_PAGE_SIZE * CLIPBOARD_PAGE_SIZE;
        }
        state.page_offset = state
            .page_offset
            .min(ids.len().saturating_sub(1) / CLIPBOARD_PAGE_SIZE * CLIPBOARD_PAGE_SIZE);
        let page =
            &ids[state.page_offset..(state.page_offset + CLIPBOARD_PAGE_SIZE).min(ids.len())];
        if state
            .selected_item_id
            .as_ref()
            .is_none_or(|id| !page.contains(id))
        {
            state.selected_item_id = page.first().cloned();
            state.text_offset = 0;
        }
        let items = self._database.items(page)?;
        let selected = state
            .selected_item_id
            .as_deref()
            .map(|id| self._database.entry(id))
            .transpose()?;
        let mut paths = HashMap::new();
        for item in &items {
            if let Some(path) = &item.first_path {
                paths.insert(item.entry_id.clone(), vec![path.clone()]);
            }
        }
        if let Some(ClipboardEntry {
            entry_id,
            content:
                nanika_protocol::ClipboardContent::Files {
                    paths: selected_paths,
                },
            ..
        }) = &selected
        {
            paths.insert(
                entry_id.clone(),
                selected_paths
                    .iter()
                    .take(crate::FILE_COLLECTION_PREVIEW_LIMIT)
                    .cloned()
                    .collect(),
            );
        }
        let view = crate::view::clipboard_view(&mut state, &items, selected.as_ref(), ids.len())?;
        view.validate()?;
        Ok(ClipboardPresentation {
            state,
            view,
            data_revision: self._revision,
            matching_ids: ids,
            paths,
        })
    }

    fn _changed(&mut self, upsert: Option<&ClipboardEntry>, change: &ClipboardChange) {
        self._revision = self._revision.wrapping_add(1);
        if let Some(query) = &mut self._query {
            query.apply(upsert, &change.removed);
        }
    }
}

#[cfg(test)]
#[path = "../tests/ClipboardStore.rs"]
mod tests;
