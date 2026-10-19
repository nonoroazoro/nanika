use super::handle_process_group_termination;

#[test]
fn process_group_termination_ignores_only_a_missing_process() {
    assert!(handle_process_group_termination(Ok(())).is_ok());
    assert!(handle_process_group_termination(Err(rustix::io::Errno::SRCH)).is_ok());
    let error = handle_process_group_termination(Err(rustix::io::Errno::PERM))
        .expect_err("permission errors must be propagated");
    assert_eq!(
        error.raw_os_error(),
        Some(rustix::io::Errno::PERM.raw_os_error())
    );
}
