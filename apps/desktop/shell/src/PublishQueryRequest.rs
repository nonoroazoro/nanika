use serde::Deserialize;

#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PublishQueryRequest {
    pub(crate) session_id: u64,
    pub(crate) request_id: u64,
    pub(crate) query: String
}

impl PublishQueryRequest {
    pub(crate) fn validate(&self) -> Result<(), nanika_host::OperationError> {
        use nanika_host::{OperationError, OperationErrorCode as Code};
        if self.request_id == 0 || self.request_id > 9_007_199_254_740_991 {
            return Err(OperationError::new(Code::InvalidSearchRequest));
        }
        if self.query.chars().count() > nanika_search::MAX_QUERY_CHARS {
            return Err(OperationError::new(Code::SearchQueryTooLong));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/PublishQueryRequest.rs"]
mod tests;
