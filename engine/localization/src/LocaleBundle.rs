use crate::{
    MAX_EXPANDED_BYTES, MAX_LOCALE_BYTES, MAX_LOCALE_RESOURCES, MAX_RESOURCE_BYTES,
    MessageContract, SupportedLocale
};
use crate::{bounded_output::BoundedOutput, resource_validator::ResourceValidator};
use fluent_bundle::{FluentArgs, FluentResource, FluentValue, concurrent::FluentBundle};
use nanika_protocol::{LocalizedText, MessageArgument};
use std::collections::BTreeMap;

/// An immutable, concurrent bundle for exactly one locale and one trusted resource owner.
pub struct LocaleBundle {
    _bundle: FluentBundle<FluentResource>,
    _contracts: BTreeMap<String, MessageContract>
}

impl LocaleBundle {
    pub fn new(
        locale: SupportedLocale,
        sources: impl IntoIterator<Item = (String, String)>
    ) -> Result<Self, String> {
        let mut resources = Vec::new();
        let mut total = 0usize;
        for (name, source) in sources {
            if source.len() > MAX_RESOURCE_BYTES
                || source.len() > MAX_LOCALE_BYTES.saturating_sub(total)
                || resources.len() >= MAX_LOCALE_RESOURCES
            {
                return Err(format!("{name}: Fluent resource budget exceeded"));
            }
            total += source.len();
            crate::source::validate_source(&source).map_err(|error| format!("{name}: {error}"))?;
            let resource = FluentResource::try_new(source)
                .map_err(|(_, errors)| format!("{name}: Fluent syntax errors: {errors:?}"))?;
            resources.push((name, resource));
        }
        if resources.is_empty() {
            return Err("Locale has no resources.".into());
        }
        let contracts = ResourceValidator::validate(&resources)?;
        let mut bundle =
            FluentBundle::new_concurrent(vec![locale.as_str().parse().expect("supported locale")]);
        // Keep numeric values for plural selection. Resource validation excludes numeric
        // literals/functions from output, and dynamic arguments are JSON-safe integers,
        // so conversion back to an ungrouped integer is exact.
        bundle.set_formatter(Some(|value, _| match value {
            FluentValue::Number(number) => Some(i64::from(number).to_string()),
            _ => None
        }));
        for (name, resource) in resources {
            bundle
                .add_resource(resource)
                .map_err(|errors| format!("{name}: {errors:?}"))?;
        }
        Ok(Self {
            _bundle: bundle,
            _contracts: contracts
        })
    }

    pub fn contracts(&self) -> &BTreeMap<String, MessageContract> {
        &self._contracts
    }

    /// Missing messages are explicit. Package-level fallback chooses a complete bundle first.
    pub fn format(
        &self,
        text: &LocalizedText,
        platform: &str,
        maximum_chars: usize,
        allow_empty: bool
    ) -> Result<String, String> {
        if !matches!(platform, "macos" | "windows") {
            return Err("Unsupported localization platform.".into());
        }
        let output = match text {
            LocalizedText::Literal(value) => {
                Self::_validate_output(value, maximum_chars, allow_empty)?;
                return Ok(value.clone());
            }
            LocalizedText::Translated(value) => {
                text.validate_literals(|value| {
                    Self::_validate_output(value, maximum_chars, allow_empty)
                })?;
                return Ok(value
                    .resolve(&self._bundle.locales[0].to_string())
                    .to_owned());
            }
            LocalizedText::Message(reference) => {
                reference.validate()?;
                let contract = self
                    ._contracts
                    .get(&reference.message)
                    .ok_or_else(|| format!("{}: unknown message", reference.message))?;
                if contract
                    .arguments
                    .iter()
                    .filter(|name| name.as_str() != "platform")
                    .ne(reference.args.keys())
                {
                    return Err(format!(
                        "{}: message argument contract mismatch",
                        reference.message
                    ));
                }
                let mut arguments = FluentArgs::new();
                arguments.set("platform", platform);
                for (name, value) in &reference.args {
                    match value {
                        MessageArgument::Text(value) => arguments.set(name, value.as_str()),
                        MessageArgument::Integer(value) => arguments.set(name, *value)
                    }
                }
                let pattern = self
                    ._bundle
                    .get_message(&reference.message)
                    .and_then(|message| message.value())
                    .ok_or_else(|| format!("{}: missing message value", reference.message))?;
                let mut errors = Vec::new();
                let mut output = BoundedOutput {
                    text: String::new(),
                    maximum_bytes: maximum_chars.saturating_mul(4).min(MAX_EXPANDED_BYTES)
                };
                let result =
                    self._bundle
                        .write_pattern(&mut output, pattern, Some(&arguments), &mut errors);
                // Diagnostics name the reference, never user-supplied argument values.
                if result.is_err() || !errors.is_empty() {
                    return Err(format!(
                        "{}: formatting failed or exceeded its output budget",
                        reference.message
                    ));
                }
                output.text
            }
        };
        Self::_validate_output(&output, maximum_chars, allow_empty)?;
        Ok(output)
    }

    pub(crate) fn _validate_output(
        value: &str,
        maximum_chars: usize,
        allow_empty: bool
    ) -> Result<(), String> {
        if value.len() > MAX_EXPANDED_BYTES
            || value.chars().count() > maximum_chars
            || (!allow_empty && value.trim().is_empty())
            || value
                .chars()
                .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(
                "Localized output is empty, oversized, or contains invalid control characters."
                    .into()
            );
        }
        Ok(())
    }
}
