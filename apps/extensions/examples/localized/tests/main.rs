use nanika_extension_package::parse_extension_manifest;
use nanika_host::{
    ExtensionInvocationOutcome, ExtensionProcess, ExtensionRuntimeSource,
    ExtensionSearchCoordinator
};
use nanika_localization::{ResourceSet, SupportedLocale, validate_view_presentation};
use nanika_protocol::{DetailContent, NavigationEffect, VIEW_OPEN_ACTION_ID, View, ViewEvent};
use nanika_search::{SearchOwner, UsageMap};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering}
};
use std::time::Duration;

#[test]
fn external_process_publishes_once_and_host_switches_retained_messages_without_restarting() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = parse_extension_manifest(include_str!("../manifest.jsonc")).unwrap();
    let resources =
        Arc::new(ResourceSet::load(root, manifest.localization.as_ref().unwrap(), false).unwrap());
    let starts = Arc::new(AtomicUsize::new(0));
    let launched = starts.clone();
    let owner = SearchOwner::spawn(UsageMap::new()).unwrap();
    let coordinator = ExtensionSearchCoordinator::new();
    coordinator
        .register_source(
            &manifest.id,
            ExtensionRuntimeSource {
                localization: Some(resources.clone()),
                activation: manifest.activation,
                live_configuration: true,
                start: Box::new(move |_| {
                    launched.fetch_add(1, Ordering::SeqCst);
                    ExtensionProcess::spawn(env!("CARGO_BIN_EXE_nanika-localization-example"))
                        .map(Into::into)
                })
            },
            owner.handle(),
            manifest.contributes,
            Default::default()
        )
        .unwrap();
    assert_eq!(starts.load(Ordering::SeqCst), 0);
    let instance = coordinator.instance_id(&manifest.id).unwrap();
    let outcome = coordinator
        .invoke(
            &manifest.id,
            instance,
            1,
            "example.files",
            VIEW_OPEN_ACTION_ID,
            ""
        )
        .unwrap()
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    let ExtensionInvocationOutcome::Completed {
        effect:
            NavigationEffect::Push {
                view_id,
                revision,
                view
            },
        ..
    } = outcome
    else {
        panic!("Expected the external example view");
    };
    validate_view_presentation(&view, Some(&resources)).unwrap();
    let retained = view.clone();
    let View::Detail { detail } = view.as_ref() else {
        panic!("Expected file detail");
    };
    let title = detail.title.as_ref().unwrap();
    for _ in 0..100 {
        for (locale, expected) in [
            (SupportedLocale::English, "report.pdf and 2 more files"),
            (
                SupportedLocale::SimplifiedChinese,
                "report.pdf和另外 2 个文件"
            )
        ] {
            assert_eq!(
                resources
                    .format(locale, title, "macos", 128, false)
                    .unwrap()
                    .replace(['\u{2068}', '\u{2069}'], ""),
                expected
            );
        }
    }
    assert_eq!(view, retained);
    let DetailContent::Files { files } = &detail.content else {
        panic!("Expected literal file content");
    };
    assert_eq!(files[0].name, "report.pdf");
    assert_eq!(files[0].path, "/Example/report.pdf");
    assert_eq!(coordinator.instance_id(&manifest.id), Some(instance));
    assert_eq!(starts.load(Ordering::SeqCst), 1);
    let resumed = coordinator
        .view_event(
            &manifest.id,
            instance,
            1,
            view_id,
            revision,
            ViewEvent::Resumed
        )
        .unwrap()
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    assert!(
        resumed.view.is_none(),
        "Resuming the unchanged example must not republish its view"
    );
    assert_eq!(resumed.effect, NavigationEffect::None);
    coordinator.shutdown();
    owner.shutdown();
}
