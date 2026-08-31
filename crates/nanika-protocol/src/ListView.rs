use serde::{Deserialize, Serialize};

use crate::{DetailView, ListLayout, ListSection, ViewFilter};

/// A searchable, optionally paginated list rendered by the host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListView {
    pub title: String,
    pub search_placeholder: String,
    pub search_text: String,
    pub layout: ListLayout,
    pub sections: Vec<ListSection>,
    pub selected_item_id: Option<String>,
    pub detail: Option<DetailView>,
    pub filter: Option<ViewFilter>,
    pub next_cursor: Option<String>,
}
