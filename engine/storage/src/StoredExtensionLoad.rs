use crate::StoredExtension;

/// Valid extension rows plus isolated metadata errors from the same load.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredExtensionLoad {
    pub extensions: Vec<StoredExtension>,
    pub errors: Vec<String>,
}
