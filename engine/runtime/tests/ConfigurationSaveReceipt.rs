use super::ConfigurationSaveReceipt;

#[test]
fn lost_completion_is_unconfirmed_without_claiming_a_save_or_rollback() {
    let (sent, received) = std::sync::mpsc::channel();
    drop(sent);
    assert_eq!(
        ConfigurationSaveReceipt(received).wait().unwrap_err(),
        crate::OperationError::new(crate::OperationErrorCode::ConfigurationUnconfirmed)
    );
}
