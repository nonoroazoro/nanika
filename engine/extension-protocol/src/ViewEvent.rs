use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ViewEvent {
    /// Host-generated refresh after an extension invalidates an open view.
    Invalidated,
    Resumed,
    SearchChanged {
        text: String,
        /// Minimum initial rows requested by the host viewport, including its prefetch region.
        minimum_items: std::num::NonZeroU32,
    },
    /// Selection addresses the full immutable collection, independently of delivered windows.
    SelectionChanged {
        collection_id: String,
        index: usize,
    },
    FilterChanged {
        filter_id: String,
        value: String,
        minimum_items: std::num::NonZeroU32,
    },
    ListRangeChanged {
        collection_id: String,
        offset: usize,
        count: std::num::NonZeroU32,
    },
    TextChunkRequested {
        text_id: String,
        index: usize,
    },
    ActionInvoked {
        invocation: crate::ActionInvocation,
        item_id: Option<String>,
        action_id: String,
    },
}

impl ViewEvent {
    /// Content reads must answer the admitted immutable identity and requested range.
    /// A structurally valid but unrelated snapshot cannot acknowledge delivered content.
    pub fn validate_response(&self, view: Option<&crate::View>) -> Result<(), String> {
        match self {
            Self::SelectionChanged {
                collection_id,
                index,
            } => {
                let Some(crate::View::List { list }) = view else {
                    return Err("selection requires a list response".into());
                };
                if &list.collection_id != collection_id
                    || list
                        .selection
                        .as_ref()
                        .is_none_or(|selection| selection.index != *index)
                {
                    return Err(
                        "selection response changed the requested collection or position".into(),
                    );
                }
            }
            Self::ListRangeChanged {
                collection_id,
                offset,
                count,
            } => {
                let Some(crate::View::List { list }) = view else {
                    return Err("list range read requires a list response".into());
                };
                if &list.collection_id != collection_id
                    || (*offset >= list.total() && !(*offset == 0 && list.total() == 0))
                {
                    return Err("list range response changed collection identity or bounds".into());
                }
                let end = offset
                    .saturating_add(count.get() as usize)
                    .min(list.total());
                let mut base = 0;
                for section in &list.sections {
                    let first = (*offset).max(base);
                    let last = end.min(base + section.total);
                    if first < last
                        && (base + section.offset > first
                            || base + section.offset + section.items.len() < last)
                    {
                        return Err("list range response does not cover the requested rows".into());
                    }
                    base += section.total;
                }
            }
            Self::TextChunkRequested { text_id, index } => {
                let detail = match view {
                    Some(crate::View::List { list }) => list.detail.as_ref(),
                    Some(crate::View::Detail { detail }) => Some(detail),
                    None => None,
                };
                if !matches!(detail.map(|detail| &detail.content),
                    Some(crate::DetailContent::Text { text_id: actual, chunk_index, .. })
                    if actual == text_id && chunk_index == index)
                {
                    return Err(
                        "text read response does not match the requested document chunk".into(),
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }
}
