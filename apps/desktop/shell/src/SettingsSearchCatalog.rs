use crate::{ExtensionLifecycle, SettingsSearchEntry, SettingsSearchTarget};
use nanika_text_search::{TextMatcher, TextQuery, normalize_query};

pub(crate) struct SettingsSearchCatalog {
    entries: Vec<SettingsSearchEntry>
}

impl SettingsSearchCatalog {
    /// Project the same platform-filtered contributions that Settings renders. Values and runtime
    /// results deliberately never enter this catalog; disabled extensions remain discoverable.
    /// Index labels and explicit keywords, not explanatory prose that admits unrelated fuzzy matches.
    pub(crate) fn new(extensions: &[ExtensionLifecycle]) -> Self {
        let mut catalog = Self {
            entries: Vec::new()
        };
        catalog._add(
            "general",
            SettingsSearchTarget::Page,
            "General",
            &[],
            "preferences settings"
        );
        for section in crate::general_settings::sections() {
            catalog._add(
                "general",
                SettingsSearchTarget::Section {
                    key: section.key.into()
                },
                section.title,
                &["General"],
                ""
            );
            for field in section.fields {
                catalog._add(
                    "general",
                    SettingsSearchTarget::Field {
                        key: field.key.as_str().into()
                    },
                    field.title,
                    &["General", section.title],
                    field.keywords
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
            "Nanika version information"
        );
        catalog
    }

    pub(crate) fn search(self, query: &str) -> Vec<SettingsSearchEntry> {
        let query = TextQuery::new(query);
        if query.normalized().is_empty() {
            return Vec::new();
        }
        let mut matcher = TextMatcher::new();
        // Search filters the settings page; it must not reorder its navigation targets.
        // Matching still uses the shared matcher, but scores do not change presentation order.
        self.entries
            .into_iter()
            .filter(|entry| matcher.score(&query, &entry.search_values).is_some())
            .collect()
    }

    fn _extension(&mut self, extension: &ExtensionLifecycle) {
        let info = &extension.info;
        self._add(&info.id, SettingsSearchTarget::Page, &info.name, &[], "");
        self._add(
            &info.id,
            SettingsSearchTarget::Enabled,
            "Enable extension",
            &[&info.name],
            "enabled disabled"
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
        // Match the renderer's first-appearance group order, then its field order.
        let mut property_groups: Vec<(&str, Vec<_>)> = Vec::new();
        for (key, property) in properties {
            let title = property
                .group
                .as_deref()
                .unwrap_or(&configuration.contribution.title);
            if let Some((_, entries)) = property_groups
                .iter_mut()
                .find(|(group, _)| *group == title)
            {
                entries.push((key, property));
            } else {
                property_groups.push((title, vec![(key, property)]));
            }
        }
        for (group, properties) in property_groups {
            for (key, property) in properties {
                self._add(
                    &info.id,
                    SettingsSearchTarget::Field { key: key.clone() },
                    &property.title,
                    &[&info.name, group],
                    ""
                );
            }
        }
        // SettingsActions renders groups in first-appearance order, including the
        // default "Actions" group, then preserves declaration order within each group.
        let mut groups: Vec<(&str, Vec<_>)> = Vec::new();
        for item in &configuration.contribution.actions {
            let title = item.action.group.as_deref().unwrap_or("Actions");
            if let Some((_, items)) = groups.iter_mut().find(|(group, _)| *group == title) {
                items.push(item);
            } else {
                groups.push((title, vec![item]));
            }
        }
        for (_, items) in groups {
            for item in items {
                self._add(
                    &info.id,
                    SettingsSearchTarget::Action {
                        key: item.action.id.clone()
                    },
                    &item.action.title,
                    &[
                        &info.name,
                        item.action.group.as_deref().unwrap_or("Actions")
                    ],
                    ""
                );
            }
        }
    }

    fn _add(
        &mut self,
        page_id: &str,
        target: SettingsSearchTarget,
        title: &str,
        breadcrumb: &[&str],
        keywords: &str
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
                .collect()
        });
    }
}
