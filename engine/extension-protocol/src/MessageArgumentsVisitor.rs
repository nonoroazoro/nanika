use serde::de::{Error, MapAccess, Visitor};

use crate::{MAX_MESSAGE_ARGUMENTS, MessageArgument, MessageArguments};

pub(super) struct MessageArgumentsVisitor;

impl<'de> Visitor<'de> for MessageArgumentsVisitor {
    type Value = MessageArguments;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an object of bounded message arguments")
    }

    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
        let mut values: Vec<(String, MessageArgument)> =
            Vec::with_capacity(map.size_hint().unwrap_or(1).min(MAX_MESSAGE_ARGUMENTS));
        while let Some(key) = map.next_key::<String>()? {
            if let Some((_, value)) = values.iter_mut().find(|(name, _)| *name == key) {
                *value = map.next_value()?;
                continue;
            }
            if values.len() == MAX_MESSAGE_ARGUMENTS {
                return Err(M::Error::custom("Too many message arguments."));
            }
            values.push((key, map.next_value()?));
        }
        Ok(values.into_iter().collect())
    }
}
