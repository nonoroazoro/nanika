use crate::{
    LocaleBundle, LocalizationResources, MAX_LOCALE_BYTES, MAX_RESOURCE_BYTES, PreparedLocale,
    PreparedResources, SupportedLocale
};
use nanika_protocol::LocalizedText;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

/// A retained package resource set. Updating a package creates a new set; open surfaces retain this one.
pub struct ResourceSet {
    _bundles: BTreeMap<SupportedLocale, LocaleBundle>,
    _prepared: Arc<PreparedResources>
}

impl std::fmt::Debug for ResourceSet {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ResourceSet")
            .field("identity", &self._prepared.identity)
            .field("default_locale", &self._prepared.default_locale)
            .finish_non_exhaustive()
    }
}

impl ResourceSet {
    /// Load from an already staged package root, before publishing or replacing any active state.
    pub fn load(
        root: &Path,
        declaration: &LocalizationResources,
        require_complete: bool
    ) -> Result<Self, String> {
        declaration.validate()?;
        let root = fs::canonicalize(root)
            .map_err(|error| format!("Localization package root: {error}"))?;
        let mut sources = BTreeMap::new();
        for (locale, paths) in &declaration.resources {
            let mut files = Vec::new();
            let mut total = 0;
            for path in paths {
                let source = Self::_read_source(&root, path)
                    .map_err(|error| format!("{}: {path}: {error}", locale.as_str()))?;
                total += source.len();
                if total > MAX_LOCALE_BYTES {
                    return Err(format!(
                        "{}: locale resources exceed the byte budget",
                        locale.as_str()
                    ));
                }
                files.push((path.clone(), source));
            }
            sources.insert(*locale, files);
        }
        Self::new(declaration.default_locale, sources, require_complete)
    }

    /// Prepare embedded host resources through the same parser, contracts, and fallback validation.
    pub fn new(
        default_locale: SupportedLocale,
        sources: BTreeMap<SupportedLocale, Vec<(String, String)>>,
        require_complete: bool
    ) -> Result<Self, String> {
        if !sources.contains_key(&default_locale) {
            return Err("Default locale has no declared resources.".into());
        }
        if require_complete && sources.len() != 2 {
            return Err(
                "Host and built-in packages require English and Simplified Chinese resources."
                    .into()
            );
        }
        let mut bundles = BTreeMap::new();
        let mut locales = BTreeMap::new();
        let mut digest = Sha256::new();
        for (locale, mut files) in sources {
            // A file-order change cannot change semantics; duplicate IDs are rejected.
            files.sort_by(|left, right| left.0.cmp(&right.0));
            let bundle = LocaleBundle::new(locale, files.clone())?;
            Self::_hash_part(&mut digest, locale.as_str());
            for (name, source) in &files {
                Self::_hash_part(&mut digest, name);
                Self::_hash_part(&mut digest, source);
            }
            locales.insert(
                locale,
                PreparedLocale {
                    sources: files.into_iter().map(|(_, source)| source).collect(),
                    contracts: bundle.contracts().clone()
                }
            );
            bundles.insert(locale, bundle);
        }
        let default = bundles
            .get(&default_locale)
            .expect("default locale checked");
        for (locale, bundle) in &bundles {
            for (id, contract) in bundle.contracts() {
                let expected = default.contracts().get(id).ok_or_else(|| {
                    format!("{}: {id}: missing from default locale", locale.as_str())
                })?;
                // Platform is injected by the host; a translation may not need its selector.
                if contract
                    .arguments
                    .iter()
                    .filter(|name| name.as_str() != "platform")
                    .ne(expected
                        .arguments
                        .iter()
                        .filter(|name| name.as_str() != "platform"))
                {
                    return Err(format!(
                        "{}: {id}: arguments differ from default locale",
                        locale.as_str()
                    ));
                }
            }
            if require_complete && bundle.contracts().len() != default.contracts().len() {
                return Err(format!(
                    "{}: incomplete built-in translations",
                    locale.as_str()
                ));
            }
        }
        Self::_hash_part(&mut digest, default_locale.as_str());
        let identity = digest
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(Self {
            _bundles: bundles,
            _prepared: Arc::new(PreparedResources {
                identity,
                default_locale,
                locales
            })
        })
    }

    pub fn prepared(&self) -> Arc<PreparedResources> {
        self._prepared.clone()
    }

    pub fn format(
        &self,
        locale: SupportedLocale,
        text: &LocalizedText,
        platform: &str,
        maximum_chars: usize,
        allow_empty: bool
    ) -> Result<String, String> {
        // A formatting error is a contract failure, not permission to mask it with another language.
        crate::format_text(
            text,
            Some(self),
            locale,
            platform,
            maximum_chars,
            allow_empty
        )
    }

    pub(crate) fn bundle_for(
        &self,
        locale: SupportedLocale,
        text: &LocalizedText
    ) -> &LocaleBundle {
        self._bundles
            .get(&locale)
            .filter(|bundle| match text {
                LocalizedText::Literal(_) | LocalizedText::Translated(_) => true,
                LocalizedText::Message(reference) => {
                    bundle.contracts().contains_key(&reference.message)
                }
            })
            .unwrap_or_else(|| {
                self._bundles
                    .get(&self._prepared.default_locale)
                    .expect("validated default locale")
            })
    }

    fn _read_source(root: &Path, relative: &str) -> Result<String, String> {
        let canonical = fs::canonicalize(root.join(relative)).map_err(|error| error.to_string())?;
        if !canonical.starts_with(root) || canonical == root {
            return Err("resource escaped its package root".into());
        }
        let metadata = fs::metadata(&canonical).map_err(|error| error.to_string())?;
        if !metadata.is_file() || metadata.len() > MAX_RESOURCE_BYTES as u64 {
            return Err("resource must be a bounded regular file".into());
        }
        let file = File::open(canonical).map_err(|error| error.to_string())?;
        let mut source = String::new();
        file.take((MAX_RESOURCE_BYTES + 1) as u64)
            .read_to_string(&mut source)
            .map_err(|error| error.to_string())?;
        if source.len() > MAX_RESOURCE_BYTES {
            return Err("resource exceeds the byte budget".into());
        }
        Ok(source)
    }

    fn _hash_part(digest: &mut Sha256, value: &str) {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value.as_bytes());
    }
}
