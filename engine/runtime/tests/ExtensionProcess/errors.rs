use super::extension_reported_error;
use crate::SupervisorError;
use nanika_localization::{ResourceSet, SupportedLocale};
use nanika_protocol::{ProtocolError, message};

#[test]
fn correlated_business_failure_retains_both_locales_and_its_diagnostic_source() {
    let resources = ResourceSet::new(
        SupportedLocale::English,
        [
            (
                SupportedLocale::English,
                vec![("en.ftl".into(), "failed = Try another file".into())]
            ),
            (
                SupportedLocale::SimplifiedChinese,
                vec![("zh.ftl".into(), "failed = 请尝试其他文件".into())]
            )
        ]
        .into(),
        true
    )
    .unwrap();
    let failure = extension_reported_error(
        "invoke",
        "action",
        Some("action"),
        ProtocolError::new("open_failed", message("failed", [])).with_detail("OS detail"),
        Some(&resources)
    );
    assert!(std::error::Error::source(&failure).is_some());
    let SupervisorError::Reported(error) = failure else {
        panic!("business failure must remain typed");
    };
    assert_eq!(
        error.messages[&SupportedLocale::English],
        "Try another file"
    );
    assert_eq!(
        error.messages[&SupportedLocale::SimplifiedChinese],
        "请尝试其他文件"
    );
    assert_eq!(error.error.detail.as_deref(), Some("OS detail"));
}

#[test]
fn foreign_correlation_and_unbound_references_remain_protocol_failures() {
    for response in [None, Some("other"), Some("action")] {
        let error = extension_reported_error(
            "invoke",
            "action",
            response,
            ProtocolError::new("open_failed", message("failed", [])),
            None
        );
        assert!(matches!(error, SupervisorError::UnexpectedMessage(_)));
        let detail = error.to_string();
        assert!(detail.contains(if response == Some("action") {
            "requires package localization resources"
        } else {
            "uncorrelated"
        }));
    }
}
