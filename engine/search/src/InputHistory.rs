use crate::normalize_history_key;

/// Bounded keyboard history with draft-query preservation.
#[derive(Debug, Clone)]
pub struct InputHistory {
    entries: Vec<String>,
    cursor: Option<usize>,
    draft: Option<String>,
    limit: usize,
}

impl InputHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursor: None,
            draft: None,
            limit,
        }
    }

    pub fn from_entries(limit: usize, entries: impl IntoIterator<Item = String>) -> Self {
        let mut history = Self::new(limit);
        for entry in entries {
            history.record(&entry);
        }
        history
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn record(&mut self, query: &str) {
        let query = query.trim();
        if query.is_empty() || self.limit == 0 {
            return;
        }
        let key = normalize_history_key(query);
        self.entries
            .retain(|entry| normalize_history_key(entry) != key);
        self.entries.push(query.to_owned());
        if self.entries.len() > self.limit {
            self.entries.drain(..self.entries.len() - self.limit);
        }
        self.reset_navigation();
    }

    pub fn older(&mut self, current_query: &str) -> Option<String> {
        if self.cursor.is_none() {
            self.draft = Some(current_query.to_owned());
        }
        let next = match self.cursor {
            Some(index) => index.checked_sub(1),
            None => self.entries.len().checked_sub(1),
        }?;
        self.cursor = Some(next);
        self.entries.get(next).cloned()
    }

    pub fn newer(&mut self) -> Option<String> {
        let current = self.cursor?;
        if current + 1 >= self.entries.len() {
            self.cursor = None;
            return Some(self.draft.take().unwrap_or_default());
        }
        self.cursor = Some(current + 1);
        self.entries.get(current + 1).cloned()
    }

    pub fn reset_navigation(&mut self) {
        self.cursor = None;
        self.draft = None;
    }
}
