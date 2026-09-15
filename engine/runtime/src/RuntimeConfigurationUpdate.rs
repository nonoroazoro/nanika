/// Runtime application acknowledgement for one saved configuration snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfigurationUpdate {
    pub extension_id: String,
    pub request_id: String,
    pub result: Result<(), String>,
}
