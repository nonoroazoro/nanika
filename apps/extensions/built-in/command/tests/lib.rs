use nanika_extension_command::command_candidate;

#[test]
fn only_explicit_shell_queries_contribute_commands() {
    assert!(command_candidate("git status").is_none());
    assert!(command_candidate(">").is_none());
    let (candidate, command) = command_candidate("> git status").expect("command candidate");
    assert_eq!(command, "git status");
    assert_eq!(candidate.aliases, ["> git status"]);
}
