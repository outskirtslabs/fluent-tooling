use std::io::Write;
use std::process::{Command, Stdio};

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fl-lint"))
}

fn fixture(path: &str) -> String {
    format!("{}/../../test/fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn valid_file_exits_zero_without_output() {
    let output = command()
        .arg("--no-color")
        .arg(fixture("projectfluent-reference/eof_empty.ftl"))
        .output()
        .expect("fl-lint must run");

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn broken_file_exits_one_with_rich_diagnostics() {
    let output = command()
        .arg("--no-color")
        .arg(fixture("regressions/broken.ftl"))
        .output()
        .expect("fl-lint must run");
    let stderr = String::from_utf8(output.stderr).expect("diagnostics must be UTF-8");

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("error[E0003]"));
    assert!(stderr.contains("expected token `}`"));
    assert!(stderr.contains("help:"));
    assert!(!stderr.contains('\u{1b}'));
}

#[test]
fn stdin_is_supported() {
    let mut child = command()
        .args(["--no-color", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("fl-lint must start");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(b"message-id =")
        .expect("write stdin");
    let output = child.wait_with_output().expect("fl-lint must finish");
    let stderr = String::from_utf8(output.stderr).expect("diagnostics must be UTF-8");

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr.contains("<stdin>"));
    assert!(stderr.contains("E0005"));
}

#[test]
fn invocation_errors_exit_two() {
    let output = command()
        .arg("--unknown-option")
        .output()
        .expect("fl-lint must run");
    let stderr = String::from_utf8(output.stderr).expect("usage must be UTF-8");

    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("Usage:"));
}
