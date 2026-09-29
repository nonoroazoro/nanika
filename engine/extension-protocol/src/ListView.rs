use serde::{Deserialize, Serialize};

use crate::{DetailView, ListLayout, ListSection, ViewFilter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListView {
    pub title: String,
    pub search_placeholder: String,
    pub search_text: String,
    pub empty_title: String,
    pub empty_description: String,
    pub layout: ListLayout,
    pub sections: Vec<ListSection>,
    /// Immutable query/order identity. Change when matching IDs or their order change,
    /// not when selection or the delivered window changes.
    pub collection_id: String,
    /// Independently delivered so moving the viewport never changes selection or actions.
    pub selection: Option<crate::ListSelection>,
    pub detail: Option<DetailView>,
    pub filter: Option<ViewFilter>,
}

impl ListView {
    pub fn item(&self, id: &str) -> Option<&crate::ListItem> {
        self.sections
            .iter()
            .flat_map(|section| &section.items)
            .find(|item| item.id == id)
            .or_else(|| {
                self.selection
                    .as_ref()
                    .filter(|selection| selection.item.id == id)
                    .map(|selection| &selection.item)
            })
    }

    pub fn total(&self) -> usize {
        self.sections.iter().map(|section| section.total).sum()
    }
}
