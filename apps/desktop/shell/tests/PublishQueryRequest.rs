use super::PublishQueryRequest;
use nanika_host::{OperationError, OperationErrorCode as Code};

#[test]
fn request_identity_and_unicode_query_limits_have_distinct_failures() {
    let mut request = PublishQueryRequest {
        session_id: 1,
        request_id: 1,
        query: "界".repeat(nanika_search::MAX_QUERY_CHARS)
    };
    assert!(request.validate().is_ok());
    request.query.push('😀');
    assert_eq!(
        request.validate(),
        Err(OperationError::new(Code::SearchQueryTooLong))
    );
    request.query.clear();
    for invalid in [0, 9_007_199_254_740_992, u64::MAX] {
        request.request_id = invalid;
        assert_eq!(
            request.validate(),
            Err(OperationError::new(Code::InvalidSearchRequest))
        );
    }
    request.request_id = 9_007_199_254_740_991;
    assert!(request.validate().is_ok());
    assert_eq!(
        serde_json::to_value(OperationError::new(Code::SearchQueryTooLong)).unwrap(),
        serde_json::json!({"kind": "host", "code": "searchQueryTooLong"})
    );
}
