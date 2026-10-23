use nanika_protocol::{
    HostServiceFailure, HostServiceFailureKind, HostServiceResponse, Message, message
};
use serde_json::json;

#[test]
fn failed_service_responses_retain_authority_outcome_and_original_diagnostics() {
    for (kind, outcome, code) in [
        (HostServiceFailureKind::Cancelled, "cancelled", "cancelled"),
        (
            HostServiceFailureKind::Rejected,
            "rejected",
            "host_service_rejected"
        ),
        (
            HostServiceFailureKind::Failed,
            "failed",
            "host_service_failed"
        ),
        (
            HostServiceFailureKind::Unconfirmed,
            "unconfirmed",
            "host_service_unconfirmed"
        )
    ] {
        let failure = HostServiceFailure::new(kind, "EACCES: /Original/私密.txt");
        let response = Message::HostResponse {
            request_id: "service-7".into(),
            parent_request_id: "invoke-7".into(),
            generation: 7,
            response: HostServiceResponse::Failed {
                failure: failure.clone()
            }
        };
        let wire = serde_json::to_value(&response).unwrap();
        assert_eq!(
            wire,
            json!({
                "type": "hostResponse", "request_id": "service-7", "parent_request_id": "invoke-7", "generation": 7,
                "response": {"service": "failed", "failure": {"kind": outcome, "detail": "EACCES: /Original/私密.txt"}}
            })
        );
        let decoded: Message = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), wire);
        assert!(!format!("{response:?}").contains("私密"));
        let error = failure.into_error("copy-failure");
        error.validate().unwrap();
        assert_eq!(error.code, code);
        assert_eq!(
            error.message,
            message("copy-failure", [("outcome".into(), outcome.into())])
        );
        assert_eq!(error.detail.as_deref(), Some("EACCES: /Original/私密.txt"));
        let mut foreign = wire;
        foreign["response"]["failure"]["scope"] = json!("host");
        assert!(serde_json::from_value::<Message>(foreign).is_err());
    }
    assert!(
        serde_json::from_value::<HostServiceFailure>(
            json!({"kind":"unknown", "detail":"original"})
        )
        .is_err()
    );
}

#[test]
fn converting_a_service_failure_preserves_diagnostic_admission_bounds() {
    for detail in [
        "x".repeat(nanika_protocol::MAX_ERROR_DETAIL_BYTES + 1),
        "original\0detail".into()
    ] {
        let error = HostServiceFailure::new(HostServiceFailureKind::Failed, detail.clone())
            .into_error("copy-failure");
        assert_eq!(error.detail.as_deref(), Some(detail.as_str()));
        assert!(error.validate().is_err());
    }
}
