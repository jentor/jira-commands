use std::process::Command;

#[test]
fn issue_shortcut_cannot_be_combined_with_subcommand() {
    let result = Command::new(env!("CARGO_BIN_EXE_jirac"))
        .args(["IT-1", "issue", "list"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr)
        .contains("Issue shortcut cannot be combined with a subcommand"));
}
