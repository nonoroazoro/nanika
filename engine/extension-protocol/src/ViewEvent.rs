use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ViewEvent {
    /// Host-generated refresh after an extension invalidates an open view.
    Invalidated,
    Resumed,
    SearchChanged {
        text: String,
    },
    SelectionChanged {
        item_id: Option<String>,
    },
    FilterChanged {
        filter_id: String,
        value: String,
    },
    LoadMore {
        cursor: String,
    },
    ActionInvoked {
        item_id: Option<String>,
        action_id: String,
    },
}
