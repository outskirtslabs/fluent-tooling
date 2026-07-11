use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fl-lint"))
}

fn fixture(path: &str) -> String {
    format!("{}/../../test/fixtures/{path}", env!("CARGO_MANIFEST_DIR"))
}

fn run_with_stdin(arguments: &[&str], source: &str) -> Output {
    let mut child = command()
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("fl-lint must start");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(source.as_bytes())
        .expect("write stdin");
    child.wait_with_output().expect("fl-lint must finish")
}

fn json_output(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout must contain a JSON document: {error}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn json_diagnostics(document: &Value) -> &[Value] {
    document["diagnostics"]
        .as_array()
        .expect("diagnostics must be an array")
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
    let output = run_with_stdin(&["--no-color", "-"], "message-id =");
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

#[test]
fn explicit_human_format_preserves_stderr_output() {
    let output = command()
        .args([
            "--format",
            "human",
            "--no-color",
            &fixture("regressions/broken.ftl"),
        ])
        .output()
        .expect("fl-lint must run");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[E0003]"));
}

#[test]
fn json_clean_file_exits_zero_with_an_empty_document() {
    let output = command()
        .args([
            "--format",
            "json",
            &fixture("projectfluent-reference/eof_empty.ftl"),
        ])
        .output()
        .expect("fl-lint must run");
    let document = json_output(&output);

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(document["schema_version"], 1);
    assert!(json_diagnostics(&document).is_empty());
}

#[test]
fn json_diagnostics_use_stdout_and_include_the_stable_schema() {
    let path = fixture("regressions/broken.ftl");
    let output = command()
        .args(["--format=json", &path])
        .output()
        .expect("fl-lint must run");
    let document = json_output(&output);
    let diagnostic = json_diagnostics(&document)
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E0003")
        .expect("broken fixture must contain E0003");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(diagnostic["path"], path);
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(diagnostic["message"], "expected token `}`");
    assert!(diagnostic["notes"].is_array());
    assert!(
        diagnostic["help"]
            .as_array()
            .is_some_and(|help| !help.is_empty())
    );

    let label = diagnostic["labels"]
        .as_array()
        .and_then(|labels| labels.iter().find(|label| label["primary"] == true))
        .expect("diagnostic must contain a primary label");
    assert!(label["message"].is_string());
    for endpoint in ["start", "end"] {
        assert!(label["span"][endpoint]["byte"].is_u64());
        assert!(label["span"][endpoint]["line"].is_u64());
        assert!(label["span"][endpoint]["column"].is_u64());
    }
}

#[test]
fn json_multiple_files_keep_each_diagnostic_path() {
    let first = fixture("regressions/broken.ftl");
    let second = fixture("fluent-tooling-structure/message_with_empty_pattern.ftl");
    let output = command()
        .args(["--format", "json", &first, &second])
        .output()
        .expect("fl-lint must run");
    let document = json_output(&output);
    let paths: Vec<_> = json_diagnostics(&document)
        .iter()
        .filter_map(|diagnostic| diagnostic["path"].as_str())
        .collect();

    assert_eq!(output.status.code(), Some(1));
    assert!(paths.contains(&first.as_str()));
    assert!(paths.contains(&second.as_str()));
}

#[test]
fn json_stdin_reports_empty_spans_at_zero_based_positions() {
    let output = run_with_stdin(&["--format=json", "-"], "message-id =");
    let document = json_output(&output);
    let diagnostic = json_diagnostics(&document)
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E0005")
        .expect("empty message must contain E0005");
    let span = &diagnostic["labels"][0]["span"];

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(diagnostic["path"], "<stdin>");
    assert_eq!(span["start"], span["end"]);
    assert_eq!(span["start"]["byte"], 12);
    assert_eq!(span["start"]["line"], 0);
    assert_eq!(span["start"]["column"], 12);
}

#[test]
fn json_columns_count_polish_text_and_emoji_as_unicode_characters() {
    let source = "message = Lubi\u{0119} \u{1f602} { $name ? }\n";
    let output = run_with_stdin(&["--format", "json", "-"], source);
    let document = json_output(&output);
    let diagnostic = json_diagnostics(&document)
        .iter()
        .find(|diagnostic| diagnostic["code"] == "E0003")
        .expect("unexpected question mark must contain E0003");
    let primary = diagnostic["labels"]
        .as_array()
        .and_then(|labels| labels.iter().find(|label| label["primary"] == true))
        .expect("E0003 must contain a primary label");

    assert_eq!(primary["span"]["start"]["byte"], 30);
    assert_eq!(primary["span"]["start"]["line"], 0);
    assert_eq!(primary["span"]["start"]["column"], 26);
    assert_eq!(primary["span"]["end"]["byte"], 31);
    assert_eq!(primary["span"]["end"]["column"], 27);
}

#[test]
fn json_preserves_warning_severity_and_code() {
    let source = concat!(
        "message = { PLURAL($people) ->\n",
        "   *[other] Other\n",
        "}\n",
    );
    let output = run_with_stdin(&["--format", "json", "-"], source);
    let document = json_output(&output);
    let warning = json_diagnostics(&document)
        .iter()
        .find(|diagnostic| diagnostic["code"] == "W1001")
        .expect("PLURAL selector must contain W1001");

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(warning["severity"], "warning");
    assert_eq!(warning["path"], "<stdin>");
}

#[test]
fn invalid_format_is_an_invocation_error_on_stderr() {
    let output = command()
        .args(["--format", "xml", "messages.ftl"])
        .output()
        .expect("fl-lint must run");

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("human or json"));
}

#[test]
fn json_io_failures_exit_two_and_keep_the_error_on_stderr() {
    let missing = fixture("does-not-exist.ftl");
    let output = command()
        .args(["--format", "json", &missing])
        .output()
        .expect("fl-lint must run");
    let document = json_output(&output);

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot read"));
    assert!(json_diagnostics(&document).is_empty());
}
