use serde::{Deserialize, Serialize};

/// Revokes source access until a fresh view republishes the document.
/// None revokes all documents; an empty list changes only presentation.
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextInvalidation {
    pub documents: Option<Vec<String>>
}

impl Default for TextInvalidation {
    fn default() -> Self {
        Self {
            documents: Some(Vec::new())
        }
    }
}

impl TextInvalidation {
    pub fn merge(&mut self, other: Self) {
        let Some(documents) = &mut self.documents else {
            return;
        };
        let Some(incoming) = other.documents else {
            self.documents = None;
            return;
        };
        for id in incoming {
            if documents.len() >= 128 || id.len() > 256 {
                self.documents = None;
                return;
            }
            if !documents.contains(&id) {
                documents.push(id);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.documents.as_ref().is_some_and(Vec::is_empty)
    }
}
