use nanika_localization::{
    ResourceSet, SupportedLocale, validate_navigation_presentation, validate_view_presentation
};
use nanika_protocol::{NavigationEffect, View};
use serde_json::json;

#[test]
fn view_labels_check_each_destination_in_both_languages_and_platforms() {
    let destinations = [
        ("/list/title", 256),
        ("/list/search_placeholder", 256),
        ("/list/empty_title", 256),
        ("/list/empty_description", 512),
        ("/list/sections/0/title", 256),
        ("/list/sections/0/items/0/title", 512),
        ("/list/sections/0/items/0/subtitle", 512),
        ("/list/selection/item/title", 512),
        ("/list/selection/item/subtitle", 512),
        ("/list/detail/title", 512),
        ("/list/detail/content/alternative_text", 512),
        ("/list/detail/metadata/0/title", 256),
        ("/list/detail/metadata/0/value", 2048),
        ("/list/filter/options/0/title", 128)
    ];
    for (pointer, limit) in destinations {
        let mut source = _list();
        *source.pointer_mut(pointer).unwrap() = json!({"message": "label"});
        let view: View = serde_json::from_value(source.clone()).unwrap();
        view.validate().unwrap();
        assert!(
            validate_view_presentation(&view, None).is_err(),
            "{pointer}"
        );
        let resources = _resources("Valid", &"界".repeat(limit));
        validate_view_presentation(&view, Some(&resources)).unwrap();
        assert_eq!(serde_json::to_value(&view).unwrap(), source);
        for (english, chinese) in [
            ("Valid".to_owned(), "界".repeat(limit + 1)),
            (
                format!(
                    "{{ $platform ->\n    [macos] Valid\n   *[windows] {}\n    }}",
                    "x".repeat(limit + 1)
                ),
                "有效".to_owned()
            )
        ] {
            let resources = _resources(&english, &chinese);
            assert!(
                validate_view_presentation(&view, Some(&resources)).is_err(),
                "{pointer}"
            );
        }
        *source.pointer_mut(pointer).unwrap() = json!({"message": "missing"});
        let view = serde_json::from_value(source).unwrap();
        assert!(
            validate_view_presentation(&view, Some(&resources)).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn pushed_views_validate_messages_without_rewriting_domain_documents() {
    let content = "{ message: user-content }";
    let document =
        nanika_protocol::TextDocumentIndex::new("document".into(), "revision".into(), content)
            .unwrap()
            .document;
    let view: View = serde_json::from_value(json!({
        "kind": "detail", "detail": {
            "title": {"message": "label"},
            "content": {"kind": "text", "value": content, "document": document, "syntax_highlighting": false},
            "metadata": [], "actions": []
        }
    })).unwrap();
    let mut effect = NavigationEffect::Push {
        view_id: "view".into(),
        revision: 1,
        view: Box::new(view)
    };
    let before = effect.clone();
    let resources = _resources("Title", "标题");
    validate_navigation_presentation(&effect, Some(&resources)).unwrap();
    assert_eq!(effect, before);
    assert!(validate_navigation_presentation(&effect, None).is_err());
    let NavigationEffect::Push { view, .. } = &mut effect else {
        unreachable!()
    };
    let View::Detail { detail } = view.as_mut() else {
        unreachable!()
    };
    detail.actions.push(nanika_protocol::Action::primary(
        "run",
        nanika_protocol::message("missing", [])
    ));
    assert!(validate_navigation_presentation(&effect, Some(&resources)).is_err());
}

#[test]
fn typed_metadata_needs_no_bundle_but_keeps_message_scope_validation() {
    let mut source = _list();
    *source.pointer_mut("/list/detail/metadata/0/value").unwrap() = json!({"bytes": 1536});
    let view: View = serde_json::from_value(source.clone()).unwrap();
    validate_view_presentation(&view, None).unwrap();
    assert_eq!(serde_json::to_value(&view).unwrap(), source);
    *source.pointer_mut("/list/detail/metadata/0/title").unwrap() = json!({"message": "label"});
    let view: View = serde_json::from_value(source.clone()).unwrap();
    assert!(validate_view_presentation(&view, None).is_err());
    validate_view_presentation(&view, Some(&_resources("Size", "大小"))).unwrap();
    *source.pointer_mut("/list/detail/metadata/0/value").unwrap() = json!({"bytes": u64::MAX});
    let view: View = serde_json::from_value(source).unwrap();
    assert!(validate_view_presentation(&view, Some(&_resources("Size", "大小"))).is_err());
}

fn _list() -> serde_json::Value {
    json!({"kind": "list", "list": {
        "title": "Title", "search_placeholder": "", "search_text": "用户输入",
        "empty_title": "Empty", "empty_description": "", "layout": "split", "collection_id": "collection",
        "sections": [{"id": "section", "title": "Section", "offset": 0, "total": 2,
            "items": [{"id": "visible", "title": "Visible", "subtitle": "", "icon": null, "actions": []}]}],
        "selection": {"index": 1, "item": {"id": "offscreen", "title": "Selected", "subtitle": "", "icon": null, "actions": []}},
        "detail": {"title": "Preview", "content": {"kind": "image", "source": {"path": format!("{}.png", "a".repeat(64))}, "alternative_text": "Image"},
            "metadata": [{"title": "Label", "value": "Value"}], "actions": []},
        "filter": {"id": "filter", "selected_value": "all", "options": [{"value": "all", "title": "All"}]}
    }})
}

fn _resources(english: &str, chinese: &str) -> ResourceSet {
    ResourceSet::new(
        SupportedLocale::English,
        [
            (
                SupportedLocale::English,
                vec![("en.ftl".into(), format!("label = {english}"))]
            ),
            (
                SupportedLocale::SimplifiedChinese,
                vec![("zh.ftl".into(), format!("label = {chinese}"))]
            )
        ]
        .into(),
        true
    )
    .unwrap()
}
