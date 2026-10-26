use crate::{ExtensionLifecycle, SettingsSearchEntry, SettingsSearchTarget};
use nanika_text_search::{TextMatcher, TextQuery, TextValue, normalize_query};

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
        let general = nanika_protocol::message("settings-general", []);
        catalog._host(
            "general",
            SettingsSearchTarget::Page,
            &general,
            &[],
            "preferences settings"
        );
        for section in crate::general_settings::sections() {
            let mut breadcrumb = vec![&general];
            if let Some(title) = &section.title {
                catalog._host(
                    "general",
                    SettingsSearchTarget::Section {
                        key: section.key.into()
                    },
                    title,
                    &breadcrumb,
                    ""
                );
                breadcrumb.push(title);
            }
            for field in section.fields {
                catalog._host(
                    "general",
                    SettingsSearchTarget::Field {
                        key: field.key.as_str().into()
                    },
                    &field.title,
                    &breadcrumb,
                    field.keywords
                );
            }
        }
        for extension in extensions {
            catalog._extension(extension);
        }
        catalog._host(
            "about",
            SettingsSearchTarget::Page,
            &nanika_protocol::message("settings-about", []),
            &[],
            "Nanika version information"
        );
        catalog
    }

    pub(crate) fn search(&self, query: &str) -> Vec<SettingsSearchEntry> {
        let query = TextQuery::new(query);
        if query.normalized().is_empty() {
            return Vec::new();
        }
        let mut matcher = TextMatcher::new();
        // Search filters the settings page; it must not reorder its navigation targets.
        // Matching still uses the shared matcher, but scores do not change presentation order.
        self.entries
            .iter()
            .filter(|entry| matcher.score(&query, &entry.search_values).is_some())
            .cloned()
            .collect()
    }

    fn _host(
        &mut self,
        page_id: &str,
        target: SettingsSearchTarget,
        title: &nanika_protocol::LocalizedText,
        breadcrumb: &[&nanika_protocol::LocalizedText],
        keywords: &str
    ) {
        // The catalog is built before matching. Both locales contribute independently of presentation.
        let resources = crate::host_resources::resources().expect("validated shipping resources");
        let mut search_values = vec![normalize_query(keywords)];
        for locale in [
            nanika_localization::SupportedLocale::English,
            nanika_localization::SupportedLocale::SimplifiedChinese
        ] {
            for text in std::iter::once(title).chain(breadcrumb.iter().copied()) {
                search_values.push(normalize_query(
                    &resources
                        .format(
                            locale,
                            text,
                            nanika_platform::target_platform().as_str(),
                            512,
                            false
                        )
                        .expect("validated host label")
                ));
            }
        }
        self.entries.push(SettingsSearchEntry {
            title_resource_scope: None,
            page_resource_scope: None,
            page_id: page_id.into(),
            target,
            title: title.clone(),
            page_title: breadcrumb.first().copied().unwrap_or(title).clone(),
            search_values: search_values.into_iter().map(TextValue::new).collect()
        });
    }

    fn _extension(&mut self, extension: &ExtensionLifecycle) {
        let info = &extension.info;
        self.entries.push(SettingsSearchEntry {
            page_id: info.id.clone(),
            target: SettingsSearchTarget::Page,
            title: info.name.clone(),
            page_title: info.name.clone(),
            title_resource_scope: info.resource_scope.clone(),
            page_resource_scope: info.resource_scope.clone(),
            search_values: info
                .name_search_values
                .iter()
                .map(|value| TextValue::new(normalize_query(value)))
                .collect()
        });
        self._extension_entry(
            info,
            SettingsSearchTarget::Enabled,
            &"Enable extension".into(),
            &[],
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
        let mut property_groups: Vec<(Option<&str>, Vec<_>)> = Vec::new();
        for (key, property) in properties {
            let title = property.group.as_ref().map(|group| group.id.as_str());
            if let Some((_, entries)) = property_groups
                .iter_mut()
                .find(|(group, _)| *group == title)
            {
                entries.push((key, property));
            } else {
                property_groups.push((title, vec![(key, property)]));
            }
        }
        for (_, properties) in property_groups {
            for (key, property) in properties {
                self._extension_entry(
                    info,
                    SettingsSearchTarget::Field { key: key.clone() },
                    &property.title,
                    &[],
                    ""
                );
            }
        }
        // SettingsActions renders groups in first-appearance order, including the
        // default "Actions" group, then preserves declaration order within each group.
        let mut groups: Vec<(&str, Vec<_>)> = Vec::new();
        for item in &configuration.contribution.actions {
            let title = item
                .action
                .group
                .as_ref()
                .map(|group| group.id.as_str())
                .unwrap_or("$actions");
            if let Some((_, items)) = groups.iter_mut().find(|(group, _)| *group == title) {
                items.push(item);
            } else {
                groups.push((title, vec![item]));
            }
        }
        for (_, items) in groups {
            for item in items {
                let mut breadcrumb = Vec::new();
                if let Some(nanika_protocol::PresentationGroup {
                    title: nanika_protocol::LocalizedText::Literal(title),
                    ..
                }) = &item.action.group
                {
                    breadcrumb.push(title.as_str());
                }
                self._extension_entry(
                    info,
                    SettingsSearchTarget::Action {
                        key: item.action.id.clone()
                    },
                    &item.action.title,
                    &breadcrumb,
                    ""
                );
            }
        }
    }

    fn _extension_entry(
        &mut self,
        info: &nanika_host::RuntimeExtensionInfo,
        target: SettingsSearchTarget,
        title: &nanika_protocol::LocalizedText,
        breadcrumb: &[&str],
        keywords: &str
    ) {
        let literal = match title {
            nanika_protocol::LocalizedText::Literal(value) => Some(value.as_str()),
            _ => None
        };
        let field_values = match &target {
            SettingsSearchTarget::Field { key } => info
                .configuration_search_values
                .get(&format!("field:{key}"))
                .map(Vec::as_slice)
                .unwrap_or_default(),
            SettingsSearchTarget::Action { key } => info
                .configuration_search_values
                .get(&format!("action:{key}"))
                .map(Vec::as_slice)
                .unwrap_or_default(),
            _ => &[]
        };
        let enabled = matches!(target, SettingsSearchTarget::Enabled);
        let mut search_values: Vec<_> = literal
            .into_iter()
            .chain(field_values.iter().map(String::as_str))
            .chain(breadcrumb.iter().copied())
            .chain(std::iter::once(keywords))
            .chain(info.name_search_values.iter().map(String::as_str))
            .filter(|value| !value.is_empty())
            .map(normalize_query)
            .collect();
        let title = if enabled {
            let text = nanika_protocol::message("settings-enable-extension-title", []);
            let resources = crate::host_resources::resources().expect("validated host resources");
            for locale in [
                nanika_localization::SupportedLocale::English,
                nanika_localization::SupportedLocale::SimplifiedChinese
            ] {
                search_values.push(normalize_query(
                    &resources
                        .format(
                            locale,
                            &text,
                            nanika_platform::target_platform().as_str(),
                            512,
                            false
                        )
                        .expect("validated host label")
                ));
            }
            text
        } else {
            title.clone()
        };
        self.entries.push(SettingsSearchEntry {
            page_id: info.id.clone(),
            target,
            title,
            page_title: info.name.clone(),
            title_resource_scope: if enabled {
                None
            } else {
                info.resource_scope.clone()
            },
            page_resource_scope: info.resource_scope.clone(),
            search_values: search_values.into_iter().map(TextValue::new).collect()
        });
    }
}

impl Default for SettingsSearchCatalog {
    fn default() -> Self {
        Self::new(&[])
    }
}
