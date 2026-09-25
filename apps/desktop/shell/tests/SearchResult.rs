use super::SearchResult;
use nanika_protocol::{IconReference, IconSource};
use nanika_search::{Candidate, CandidateKind};

#[test]
fn results_inherit_package_icons_and_allow_item_overrides() {
    let candidate = Candidate::new(
        CandidateKind::Action,
        "test.extension",
        "result",
        "Result",
        "copy",
        vec![nanika_protocol::Action::primary("copy", "Copy")],
        Vec::new(),
    );
    for path in ["assets/icon.png", "images/custom.png"] {
        let icon = IconSource::Package { path: path.into() };
        let result = SearchResult::from_candidate(&candidate, Some(icon));
        assert!(
            result
                .icon_url
                .unwrap()
                .ends_with(&format!("/test.extension/package/{path}"))
        );
    }
    for (icon, suffix) in [
        (
            IconSource::Package {
                path: "assets/item.png".into(),
            },
            "/test.extension/package/assets/item.png",
        ),
        (
            IconSource::Cache(IconReference::new("application-icon").unwrap()),
            "/test.extension/cache/application-icon/128.png",
        ),
    ] {
        let item = candidate.clone().with_icon(Some(icon));
        let result = SearchResult::from_candidate(
            &item,
            Some(IconSource::Package {
                path: "assets/icon.png".into(),
            }),
        );
        assert!(result.icon_url.unwrap().ends_with(suffix));
    }
}
