use serde::{Deserialize, Serialize};

/// Opaque adjacent cursors authorize replacement of a bounded page, never cumulative append.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewPagination {
    pub label: String,
    pub previous_cursor: Option<String>,
    pub next_cursor: Option<String>,
}

impl ViewPagination {
    pub fn allows(&self, cursor: &str) -> bool {
        self.previous_cursor.as_deref() == Some(cursor)
            || self.next_cursor.as_deref() == Some(cursor)
    }
}
