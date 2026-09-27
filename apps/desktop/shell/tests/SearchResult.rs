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
        let Some(crate::result_icon::ResultIcon::Image { url }) = result.icon else {
            panic!("package icon must remain an image");
        };
        assert!(url.ends_with(&format!("/test.extension/package/{path}")));
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
        let Some(crate::result_icon::ResultIcon::Image { url }) = result.icon else {
            panic!("native and package icons must remain images");
        };
        assert!(url.ends_with(suffix));
    }
    let empty = candidate.with_icon(Some(IconSource::Empty));
    let result = SearchResult::from_candidate(
        &empty,
        Some(IconSource::Package {
            path: "assets/icon.png".into(),
        }),
    );
    assert!(
        result.icon.is_none(),
        "empty icons suppress manifest inheritance"
    );
}

#[test]
fn result_subtitles_preserve_declared_layout_semantics() {
    for (subtitle, kind) in [
        (
            nanika_protocol::CandidateSubtitle::Label("Application".into()),
            "label",
        ),
        (
            nanika_protocol::CandidateSubtitle::Description("Application".into()),
            "description",
        ),
    ] {
        let candidate = Candidate::new(
            CandidateKind::Action,
            "test.extension",
            "entry",
            "Title",
            "run",
            vec![nanika_protocol::Action::primary("run", "Run")],
            Vec::new(),
        )
        .with_subtitle(Some(subtitle));
        let result = SearchResult::from_candidate(&candidate, None);
        assert_eq!(
            serde_json::to_value(result).unwrap()["subtitle"],
            serde_json::json!({"kind": kind, "text": "Application"})
        );
    }
}

#[test]
fn root_confirmation_is_exposed_only_for_the_enabled_primary_action() {
    let mut action = nanika_protocol::Action::primary("power", "Shut Down");
    action.style = nanika_protocol::ActionStyle::Destructive;
    action.allow_default_execution = false;
    action.confirmation_title = Some("Confirm Shut Down".into());
    for enabled in [true, false] {
        action.enabled = enabled;
        let candidate = Candidate::new(
            CandidateKind::Action,
            "test.system",
            "shutdown",
            "Shut Down",
            "power",
            vec![action.clone()],
            Vec::new(),
        );
        let result = SearchResult::from_candidate(&candidate, None);
        assert!(!result.allow_default_execution);
        assert_eq!(
            result.confirmation_title.as_deref(),
            enabled.then_some("Confirm Shut Down")
        );
    }
}

#[test]
fn native_images_take_priority_over_semantic_action_symbols() {
    let mut action = nanika_protocol::Action::primary("power", "Power");
    action.icon = Some(nanika_protocol::ActionIcon::Power);
    let candidate = Candidate::new(
        CandidateKind::Action,
        "test",
        "power",
        "Power",
        "power",
        vec![action],
        vec![],
    );
    let package = Some(IconSource::Package {
        path: "assets/icon.png".into(),
    });
    let result = SearchResult::from_candidate(&candidate, package.clone());
    assert!(matches!(
        result.icon,
        Some(crate::result_icon::ResultIcon::Symbol {
            name: nanika_protocol::ActionIcon::Power
        })
    ));
    let native = candidate.with_icon(Some(IconSource::Cache(
        IconReference::new("native").unwrap(),
    )));
    let result = SearchResult::from_candidate(&native, package);
    assert!(matches!(
        result.icon,
        Some(crate::result_icon::ResultIcon::Image { .. })
    ));
}
