use std::ops::Index;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::MessageArgument;

#[path = "MessageArgumentsVisitor.rs"]
mod visitor;

/// Small, ordered message arguments. A boxed slice avoids a full tree node for
/// the common zero- or one-argument reference while retaining JSON object semantics.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MessageArguments {
    _values: Box<[(String, MessageArgument)]>
}

impl MessageArguments {
    pub fn len(&self) -> usize {
        self._values.len()
    }

    pub fn is_empty(&self) -> bool {
        self._values.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self._values.iter().map(|(key, _)| key)
    }

    pub fn get(&self, key: &str) -> Option<&MessageArgument> {
        self._values
            .binary_search_by(|(name, _)| name.as_str().cmp(key))
            .ok()
            .map(|index| &self._values[index].1)
    }
}

impl<'a> IntoIterator for &'a MessageArguments {
    type Item = &'a (String, MessageArgument);
    type IntoIter = std::slice::Iter<'a, (String, MessageArgument)>;

    fn into_iter(self) -> Self::IntoIter {
        self._values.iter()
    }
}

impl FromIterator<(String, MessageArgument)> for MessageArguments {
    fn from_iter<T: IntoIterator<Item = (String, MessageArgument)>>(iter: T) -> Self {
        let mut values = iter.into_iter().collect::<Vec<_>>();
        // Stable sorting and retaining the last value match the previous map contract.
        values.sort_by(|left, right| left.0.cmp(&right.0));
        values.dedup_by(|current, previous| {
            if current.0 != previous.0 {
                return false;
            }
            std::mem::swap(&mut current.1, &mut previous.1);
            true
        });
        Self {
            _values: values.into_boxed_slice()
        }
    }
}

impl Index<&str> for MessageArguments {
    type Output = MessageArgument;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("no entry found for key")
    }
}

impl Serialize for MessageArguments {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self._values.iter().map(|(key, value)| (key, value)))
    }
}

impl<'de> Deserialize<'de> for MessageArguments {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(visitor::MessageArgumentsVisitor)
    }
}
