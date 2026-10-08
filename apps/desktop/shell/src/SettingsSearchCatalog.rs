use crate::{ExtensionLifecycle, SettingsSearchEntry, SettingsSearchTarget};
use nanika_text_search::{TextMatcher, TextQuery, normalize_query};

pub(crate) struct SettingsSearchCatalog {
    entries: Vec<SettingsSearchEntry>,
}

impl SettingsSearchCatalog {
    /// Project the same platform-filtered contributions that Settings renders. Values and runtime
    /// results deliberately never enter this catalog; disabled extensions remain discoverable.
    /// Index labels and explicit keywords, not explanatory prose that admits unrelated fuzzy matches.
    pub(crate) fn new(extensions: &[ExtensionLifecycle]) -> Self {
        let mut catalog = Self {
            entries: Vec::new(),
        };
        catalog._add(
            "general",
            SettingsSearchTarget::Page,
            "General",
            &[],
            "preferences settings",
        );
        for section in crate::general_settings::sections() {
            catalog._add(
                "general",
                SettingsSearchTarget::Section {
                    key: section.key.into(),
                },
                section.title,
                &["General"],
                "",
            );
            for field in section.fields {
                catalog._add(
                    "general",
                    SettingsSearchTarget::Field {
                        key: field.key.as_str().into(),
                    },
                    field.title,
                    &["General", section.title],
                    field.keywords,
                );
            }
        }
        for extension in extensions {
            catalog._extension(extension);
        }
        catalog._add(
            "about",
            SettingsSearchTarget::Page,
            "About",
            &[],
            "Nanika version information",
        );
        catalog
    }

    pub(crate) fn search(self, query: &str) -> Vec<SettingsSearchEntry> {
        let query = TextQuery::new(query);
        if query.normalized().is_empty() {
            return Vec::new();
        }
        let mut matcher = TextMatcher::new();
        let mut ranked: Vec<_> = self
            .entries
            .into_iter()
            .filter_map(|entry| {
                matcher
                    .score(&query, &entry.search_values)
                    .map(|score| (score, entry))
            })
            .collect();
        // Stable sort keeps presentation order as the tie-break. No arbitrary result truncation.
        ranked.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        ranked.into_iter().map(|(_, entry)| entry).collect()
    }

    fn _extension(&mut self, extension: &ExtensionLifecycle) {
        let info = &extension.info;
        self._add(&info.id, SettingsSearchTarget::Page, &info.name, &[], "");
        self._add(
            &info.id,
            SettingsSearchTarget::Enabled,
            "Enable extension",
            &[&info.name],
            "enabled disabled",
        );
        // An invalid configuration has no editable rows in the renderer.
        if info.configuration_error.is_some() {
            return;
        }
        let Some(configuration) = &extension.configuration else {
            return;
        };
        let mut properties: Vec<_> = configuration.contribution.properties.iter().collect();
        properties.sort_by(|(left_key, left), (right_key, right)| {
            left.order
                .cmp(&right.order)
                .then_with(|| left_key.cmp(right_key))
        });
        for (key, property) in properties {
            self._add(
                &info.id,
                SettingsSearchTarget::Field { key: key.clone() },
                &property.title,
                &[&info.name],
                "",
            );
        }
    }

    fn _add(
        &mut self,
        page_id: &str,
        target: SettingsSearchTarget,
        title: &str,
        breadcrumb: &[&str],
        keywords: &str,
    ) {
        self.entries.push(SettingsSearchEntry {
            page_id: page_id.into(),
            target,
            title: title.into(),
            page_title: breadcrumb.first().copied().unwrap_or(title).to_owned(),
            search_values: std::iter::once(title)
                .chain(breadcrumb.iter().copied())
                .chain(std::iter::once(keywords))
                .filter(|value| !value.is_empty())
                .map(normalize_query)
                .collect(),
        });
    }
}
