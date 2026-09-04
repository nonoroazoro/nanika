use serde::{Deserialize, Serialize};

/// One user interaction sent to the extension that owns a view session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ViewEvent {
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
