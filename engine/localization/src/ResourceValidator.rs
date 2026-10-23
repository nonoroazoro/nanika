use crate::{MAX_EXPANDED_BYTES, MAX_REFERENCE_DEPTH, MAX_RESOURCE_ENTRIES, MessageContract};
use fluent_bundle::FluentResource;
use fluent_syntax::ast::{
    Entry, Expression, InlineExpression, Pattern, PatternElement, VariantKey
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct ResourceValidator<'a> {
    _patterns: BTreeMap<String, (&'a str, &'a Pattern<&'a str>)>,
    _contracts: BTreeMap<String, MessageContract>,
    _visiting: BTreeSet<String>
}

impl<'a> ResourceValidator<'a> {
    pub(crate) fn validate(
        resources: &'a [(String, FluentResource)]
    ) -> Result<BTreeMap<String, MessageContract>, String> {
        let mut identifiers = BTreeSet::new();
        let mut validator = Self {
            _patterns: BTreeMap::new(),
            _contracts: BTreeMap::new(),
            _visiting: BTreeSet::new()
        };
        for (source, resource) in resources {
            for entry in resource.entries() {
                let (id, pattern, attributes) = match entry {
                    Entry::Message(message) => (
                        message.id.name.to_owned(),
                        message.value.as_ref().ok_or_else(|| {
                            format!("{source}: {}: message value is required", message.id.name)
                        })?,
                        &message.attributes
                    ),
                    Entry::Term(term) => {
                        (format!("-{}", term.id.name), &term.value, &term.attributes)
                    }
                    Entry::Comment(_) | Entry::GroupComment(_) | Entry::ResourceComment(_) => {
                        continue;
                    }
                    Entry::Junk { .. } => return Err(format!("{source}: invalid Fluent syntax"))
                };
                if !nanika_protocol::is_message_identifier(id.trim_start_matches('-'))
                    || !attributes.is_empty()
                {
                    return Err(format!(
                        "{source}: {id}: unsupported identifier or attributes"
                    ));
                }
                if !identifiers.insert(id.trim_start_matches('-').to_owned())
                    || validator
                        ._patterns
                        .insert(id.clone(), (source, pattern))
                        .is_some()
                {
                    return Err(format!("{source}: {id}: duplicate identifier"));
                }
                if validator._patterns.len() > MAX_RESOURCE_ENTRIES {
                    return Err(format!("{source}: too many Fluent entries"));
                }
            }
        }
        for id in validator._patterns.keys().cloned().collect::<Vec<_>>() {
            validator._contract(&id)?;
        }
        validator._contracts.retain(|id, _| !id.starts_with('-'));
        Ok(validator._contracts)
    }

    fn _contract(&mut self, id: &str) -> Result<MessageContract, String> {
        if let Some(contract) = self._contracts.get(id) {
            return Ok(contract.clone());
        }
        if self._visiting.len() >= MAX_REFERENCE_DEPTH || !self._visiting.insert(id.to_owned()) {
            return Err(format!("{id}: cyclic or too deeply nested reference"));
        }
        let (source, pattern) = self
            ._patterns
            .get(id)
            .copied()
            .ok_or_else(|| format!("{id}: unknown static reference"))?;
        let mut contract = MessageContract::default();
        self._pattern(pattern, &mut contract)
            .map_err(|error| format!("{source}: {id}: {error}"))?;
        if contract.arguments.len()
            > nanika_protocol::MAX_MESSAGE_ARGUMENTS
                + usize::from(contract.arguments.contains("platform"))
        {
            return Err(format!("{source}: {id}: too many arguments"));
        }
        self._visiting.remove(id);
        self._contracts.insert(id.to_owned(), contract.clone());
        Ok(contract)
    }

    fn _pattern(
        &mut self,
        pattern: &Pattern<&str>,
        contract: &mut MessageContract
    ) -> Result<(), String> {
        for element in &pattern.elements {
            match element {
                PatternElement::TextElement { value } => {
                    Self::_text(value)?;
                    Self::_add_bytes(contract, value.len())?;
                }
                PatternElement::Placeable { expression } => {
                    Self::_add_placeables(contract, 1)?;
                    Self::_add_bytes(contract, 6)?;
                    self._expression(expression, contract)?;
                }
            }
        }
        Ok(())
    }

    fn _expression(
        &mut self,
        expression: &Expression<&str>,
        contract: &mut MessageContract
    ) -> Result<(), String> {
        match expression {
            Expression::Inline(value) => self._inline(value, contract),
            Expression::Select { selector, variants } => {
                self._inline(selector, contract)?;
                let mut largest = 0;
                let mut placeables = 0;
                for variant in variants {
                    if let VariantKey::NumberLiteral { value } = variant.key {
                        Self::_integer(value)?;
                    }
                    let mut branch = MessageContract::default();
                    self._pattern(&variant.value, &mut branch)?;
                    largest = largest.max(branch.expanded_bytes);
                    placeables = placeables.max(branch.placeables);
                    contract.arguments.extend(branch.arguments);
                    contract.depth = contract.depth.max(branch.depth);
                }
                Self::_add_placeables(contract, placeables)?;
                Self::_add_bytes(contract, largest)
            }
        }
    }

    fn _inline(
        &mut self,
        expression: &InlineExpression<&str>,
        contract: &mut MessageContract
    ) -> Result<(), String> {
        match expression {
            InlineExpression::StringLiteral { value } => {
                Self::_literal(value)?;
                Self::_add_bytes(contract, value.len())
            }
            InlineExpression::NumberLiteral { .. } => {
                Err("Numeric literals are selector keys only; pass output as an argument.".into())
            }
            InlineExpression::VariableReference { id } => {
                if !nanika_protocol::is_message_identifier(id.name) {
                    return Err("invalid variable identifier".into());
                }
                contract.arguments.insert(id.name.to_owned());
                Self::_add_bytes(contract, nanika_protocol::MAX_MESSAGE_ARGUMENT_BYTES)
            }
            InlineExpression::MessageReference {
                id,
                attribute: None
            } => {
                let dependency = self._contract(id.name)?;
                self._merge(contract, dependency)
            }
            InlineExpression::TermReference {
                id,
                attribute: None,
                arguments
            } => {
                let dependency = self._term(id.name, arguments.as_ref())?;
                self._merge(contract, dependency)
            }
            InlineExpression::Placeable { .. } => {
                Err("nested inline placeables are unsupported".into())
            }
            _ => Err("Fluent functions and attribute references are unsupported".into())
        }
    }

    fn _term(
        &mut self,
        name: &str,
        arguments: Option<&fluent_syntax::ast::CallArguments<&str>>
    ) -> Result<MessageContract, String> {
        let mut dependency = self._contract(&format!("-{name}"))?;
        let mut supplied = BTreeSet::new();
        if let Some(arguments) = arguments {
            if !arguments.positional.is_empty() {
                return Err("positional term arguments are unsupported".into());
            }
            for argument in &arguments.named {
                if argument.name.name == "platform"
                    || !supplied.insert(argument.name.name.to_owned())
                {
                    return Err("duplicate or reserved term argument".into());
                }
                match &argument.value {
                    InlineExpression::StringLiteral { value }
                        if value.len() <= nanika_protocol::MAX_MESSAGE_ARGUMENT_BYTES =>
                    {
                        Self::_literal(value)?
                    }
                    _ => return Err("term arguments must be bounded string literals".into())
                }
            }
        }
        if supplied != dependency.arguments {
            return Err(format!("-{name}: term argument contract mismatch"));
        }
        // Fluent term parameters have local scope, not the caller's variable scope.
        dependency.arguments.clear();
        Ok(dependency)
    }

    fn _merge(
        &self,
        contract: &mut MessageContract,
        dependency: MessageContract
    ) -> Result<(), String> {
        contract.depth = contract.depth.max(dependency.depth + 1);
        if contract.depth >= MAX_REFERENCE_DEPTH {
            return Err("reference depth exceeds the limit".into());
        }
        contract.arguments.extend(dependency.arguments);
        Self::_add_placeables(contract, dependency.placeables)?;
        Self::_add_bytes(contract, dependency.expanded_bytes)
    }

    fn _add_bytes(contract: &mut MessageContract, count: usize) -> Result<(), String> {
        if count > MAX_EXPANDED_BYTES.saturating_sub(contract.expanded_bytes) {
            return Err("expanded output exceeds the resource budget".into());
        }
        contract.expanded_bytes += count;
        Ok(())
    }

    fn _add_placeables(contract: &mut MessageContract, count: usize) -> Result<(), String> {
        // Both pinned runtimes stop after 100 expanded placeables. Reject every offending branch at admission.
        if count > 100usize.saturating_sub(contract.placeables) {
            return Err("expanded placeable count exceeds the limit".into());
        }
        contract.placeables += count;
        Ok(())
    }

    fn _literal(value: &str) -> Result<(), String> {
        let bytes = value.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                let digits = match bytes.get(index + 1) {
                    Some(b'u') => 4,
                    Some(b'U') => 6,
                    _ => 0
                };
                if digits > 0 {
                    let scalar = value
                        .get(index + 2..index + 2 + digits)
                        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                        .and_then(char::from_u32);
                    if scalar.is_none() {
                        return Err("invalid Unicode scalar escape".into());
                    }
                    index += digits;
                }
                index += 2;
            } else {
                index += 1;
            }
        }
        Self::_text(&fluent_syntax::unicode::unescape_unicode_to_string(value))
    }

    fn _text(value: &str) -> Result<(), String> {
        if value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            Err("resource contains invalid control characters".into())
        } else {
            Ok(())
        }
    }

    fn _integer(value: &str) -> Result<(), String> {
        if value.parse::<i64>().is_ok_and(|value| {
            (-nanika_protocol::MAX_MESSAGE_INTEGER..=nanika_protocol::MAX_MESSAGE_INTEGER)
                .contains(&value)
        }) {
            Ok(())
        } else {
            Err("numeric literals must be safe integers".into())
        }
    }
}
