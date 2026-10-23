use std::sync::OnceLock;

use nanika_localization::{ResourceSet, SupportedLocale};

/// Embedded host resources use the same validation and immutable bundles as extension packages.
pub(crate) fn resources() -> Result<&'static ResourceSet, String> {
    static RESOURCES: OnceLock<Result<ResourceSet, String>> = OnceLock::new();
    RESOURCES
        .get_or_init(|| {
            ResourceSet::new(
                SupportedLocale::English,
                [
                    (
                        SupportedLocale::English,
                        vec![(
                            "locales/en.ftl".into(),
                            include_str!("../locales/en.ftl").into()
                        )]
                    ),
                    (
                        SupportedLocale::SimplifiedChinese,
                        vec![(
                            "locales/zh-Hans.ftl".into(),
                            include_str!("../locales/zh-Hans.ftl").into()
                        )]
                    )
                ]
                .into(),
                true
            )
        })
        .as_ref()
        .map_err(Clone::clone)
}
