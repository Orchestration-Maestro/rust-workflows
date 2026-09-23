#![forbid(unsafe_code)]

#[test]
fn prints_checked_sum() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_workspace-sum"))
        .output()
        .expect("example binary must run");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"42\n");
    assert!(output.stderr.is_empty());
}
