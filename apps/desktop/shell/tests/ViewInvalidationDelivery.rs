use super::_accept_refresh_effect;
use nanika_host::{OperationError, OperationErrorCode as Code};
use nanika_protocol::{NavigationEffect, View};

#[test]
fn rejected_refresh_navigation_closes_only_new_views_and_retains_cleanup_failure() {
    let pushed = NavigationEffect::Push {
        view_id: "new-view".into(),
        revision: 7,
        view: Box::new(View::Detail {
            detail: nanika_protocol::DetailView {
                title: None,
                content: nanika_protocol::DetailContent::Files { files: Vec::new() },
                metadata: Vec::new(),
                actions: Vec::new()
            }
        })
    };
    for effect in [
        NavigationEffect::None,
        NavigationEffect::Pop,
        NavigationEffect::Dismiss,
        pushed.clone()
    ] {
        let result =
            _accept_refresh_effect(&effect, true, |_, _| panic!("cannot close an owned view"));
        assert_eq!(result.is_ok(), effect == NavigationEffect::None);
    }
    let cleanup = OperationError::Extension {
        error: Box::new(
            nanika_localization::PreparedError::new(
                nanika_protocol::ProtocolError::new("close_failed", "Original close explanation")
                    .with_detail("EACCES: original"),
                None,
                "macos"
            )
            .unwrap()
        )
    };
    let mut closed = None;
    let error = _accept_refresh_effect(&pushed, false, |id, revision| {
        closed = Some((id.to_owned(), revision));
        Err(cleanup.clone())
    })
    .unwrap_err();
    assert_eq!(closed, Some(("new-view".into(), 7)));
    let OperationError::Cleanup {
        primary,
        cleanup: retained
    } = error
    else {
        panic!("both failures must survive")
    };
    assert!(matches!(
        *primary,
        OperationError::Host {
            code: Code::ExtensionProtocol,
            ..
        }
    ));
    assert_eq!(*retained, cleanup);
    let error = _accept_refresh_effect(&pushed, false, |id, revision| {
        assert_eq!((id, revision), ("new-view", 7));
        Ok(())
    })
    .unwrap_err();
    assert!(matches!(
        error,
        OperationError::Host {
            code: Code::ExtensionProtocol,
            ..
        }
    ));
}
