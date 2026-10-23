use std::sync::Arc;

use crate::{ContextMenuRequest, ExtensionViewSnapshot, MenuTarget, SearchSession};
use nanika_protocol::{
    Action, ActionStyle, DetailContent, DetailView, ListItem, ListLayout, ListSection, ListView,
    View
};
use nanika_search::{Candidate, CandidateKind, RankedCandidate, SearchSnapshot};

#[test]
fn root_menus_exclude_direct_activation_by_identity_and_keep_single_extras() {
    let direct = Action::primary("execute", "Localized action");
    let extra = _extra("inspect", "Open");
    for actions in [
        vec![direct.clone()],
        vec![direct.clone(), extra.clone()],
        vec![extra.clone(), direct.clone()]
    ] {
        let (session, request) = _root(actions.clone());
        let menu = session.menu_actions(&request).unwrap();
        assert_eq!(
            menu,
            actions
                .into_iter()
                .filter(|action| action.id != "execute")
                .collect::<Vec<_>>()
        );
    }
    let mut dangerous = direct.clone();
    dangerous.style = ActionStyle::Destructive;
    dangerous.allow_default_execution = false;
    dangerous.confirmation_title = Some("Confirm".into());
    let (session, request) = _root(vec![dangerous]);
    assert!(session.menu_actions(&request).unwrap().is_empty());
    let mut explicit = direct;
    explicit.allow_default_execution = false;
    let (session, request) = _root(vec![explicit.clone()]);
    assert_eq!(
        session.menu_actions(&request).unwrap(),
        [explicit],
        "an action without a direct entry remains reachable"
    );
}

#[test]
fn list_and_detail_menus_exclude_the_same_action_as_enter_and_row_activation() {
    let direct = Action::primary("execute", "Copy");
    let extra = _extra("inspect", "Open");
    for actions in [
        vec![direct.clone()],
        vec![direct, extra.clone()],
        vec![extra.clone()]
    ] {
        let expected = actions
            .iter()
            .filter(|action| action.id != "execute")
            .cloned()
            .collect::<Vec<_>>();
        let views = [
            (
                View::Detail {
                    detail: DetailView {
                        title: None,
                        content: DetailContent::Text {
                            value: "test".into(),
                            document: nanika_protocol::TextDocumentIndex::new(
                                "test.text".into(),
                                "fixture".into(),
                                "test"
                            )
                            .unwrap()
                            .document,
                            syntax_highlighting: false
                        },
                        metadata: vec![],
                        actions: actions.clone()
                    }
                },
                None
            ),
            (
                View::List {
                    list: Box::new(ListView {
                        title: "Test".into(),
                        search_placeholder: "Search".into(),
                        empty_title: "No items".into(),
                        empty_description: "Items will appear here when available.".into(),
                        search_text: String::new(),
                        layout: ListLayout::Split,
                        sections: vec![ListSection {
                            id: "items".into(),
                            title: None,
                            offset: 0,
                            total: 1,
                            items: vec![ListItem {
                                id: "item".into(),
                                title: "Test".into(),
                                subtitle: None,
                                icon: None,
                                actions
                            }]
                        }],
                        collection_id: "test.collection".into(),
                        selection: None,
                        detail: None,
                        filter: None
                    })
                },
                Some("item".into())
            )
        ];
        for (view, item_id) in views {
            let mut session = _session();
            session.navigation.stack.push(ExtensionViewSnapshot {
                resource_scope: None,
                route_id: 1,
                extension_id: "test.extension".into(),
                extension_name: "Test extension".into(),
                extension_icon_url: "icon.png".into(),
                instance_id: 1,
                generation: 1,
                view_id: "view".into(),
                revision: 2,
                view: Arc::new(view)
            });
            let menu = crate::ConfigurationMenuRequest {
                session_id: 1,
                route_id: 1,
                instance_id: 1
            };
            assert_eq!(
                session
                    .configuration_menu_route(&menu)
                    .unwrap()
                    .extension_id,
                "test.extension"
            );
            for request in [
                crate::ConfigurationMenuRequest {
                    session_id: 2,
                    ..menu.clone()
                },
                crate::ConfigurationMenuRequest {
                    route_id: 2,
                    ..menu.clone()
                },
                crate::ConfigurationMenuRequest {
                    instance_id: 2,
                    ..menu.clone()
                }
            ] {
                assert!(session.configuration_menu_route(&request).is_err());
            }
            session.navigation.busy = true;
            assert!(session.configuration_menu_route(&menu).is_err());
            session.navigation.busy = false;
            let mut request = ContextMenuRequest {
                session_id: 1,
                target: MenuTarget::View {
                    route_id: 1,
                    revision: 2,
                    item_id: item_id.clone()
                }
            };
            assert_eq!(session.menu_actions(&request).unwrap(), expected);
            request.target = MenuTarget::View {
                route_id: 1,
                revision: 1,
                item_id
            };
            assert!(
                session.menu_actions(&request).is_err(),
                "stale menu targets must fail"
            );
        }
    }
}

#[test]
fn menus_reject_stale_sessions_rankings_and_busy_navigation() {
    let (mut session, mut request) = _root(vec![
        Action::primary("execute", "Open"),
        _extra("inspect", "Inspect"),
    ]);
    request.session_id = 2;
    assert_eq!(
        session.menu_actions(&request).err(),
        Some(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::SessionUnavailable
        ))
    );
    request.session_id = 1;
    session.result_revision += 1;
    assert_eq!(
        session.menu_actions(&request).err(),
        Some(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::SearchChanged
        ))
    );
    session.result_revision -= 1;
    session.navigation.busy = true;
    assert_eq!(
        session.menu_actions(&request).err(),
        Some(nanika_host::OperationError::new(
            nanika_host::OperationErrorCode::OperationPending
        ))
    );
}

fn _extra(id: &str, title: &str) -> Action {
    let mut action = Action::primary(id, title);
    action.style = ActionStyle::Secondary;
    action.allow_default_execution = false;
    action
}

fn _session() -> SearchSession {
    SearchSession::new(1, tauri::ipc::Channel::new(|_| Ok(())))
}

fn _root(actions: Vec<Action>) -> (SearchSession, ContextMenuRequest) {
    let mut session = _session();
    session.delivered = Some(Arc::new(SearchSnapshot {
        result_revision: 1,
        instances: Default::default(),
        generation: 0,
        normalized_query: String::new(),
        pending_extensions: Vec::new(),
        results: vec![RankedCandidate {
            candidate: Candidate::new(
                CandidateKind::Action,
                "test.extension",
                "entry",
                "Test",
                "execute",
                actions,
                vec![]
            ),
            lexical_tier: 0,
            fuzzy_score: 0,
            contextual_boost: 0
        }]
        .into()
    }));
    (
        session,
        ContextMenuRequest {
            session_id: 1,
            target: MenuTarget::Search {
                request_id: 0,
                result_revision: 0,
                extension_id: "test.extension".into(),
                entry_id: "entry".into()
            }
        }
    )
}

#[test]
fn root_menus_survive_transport_updates_but_not_query_or_result_changes() {
    let (mut session, request) = _root(vec![
        Action::primary("execute", "Open"),
        _extra("inspect", "Inspect"),
    ]);
    session.revision += 10;
    session.result_range = (1, 64);
    session.range_id += 1;
    assert_eq!(session.menu_actions(&request).unwrap().len(), 1);
    session.request_id += 1;
    assert!(session.menu_actions(&request).is_err());
    session.request_id -= 1;
    session.result_revision += 1;
    assert!(session.menu_actions(&request).is_err());
}
