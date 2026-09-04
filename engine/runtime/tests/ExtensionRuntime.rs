use crate::acp_prompt;

#[test]
fn acp_activation_requires_an_exact_extension_prefix_and_prompt() {
    let extension_id = "com.example.agent";

    assert_eq!(
        acp_prompt(extension_id, "@com.example.agent explain this"),
        Some("explain this")
    );
    assert_eq!(
        acp_prompt(extension_id, "  @com.example.agent   explain this  "),
        Some("explain this")
    );
    assert_eq!(acp_prompt(extension_id, "explain this"), None);
    assert_eq!(acp_prompt(extension_id, "@com.example.agent"), None);
    assert_eq!(
        acp_prompt(extension_id, "@com.example.agent-extra prompt"),
        None
    );
}
