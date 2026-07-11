use fluent_lint::{Diagnostic, Severity, lint, render_plain};
use serde_json::Value;

fn diagnostic<'a>(diagnostics: &'a [Diagnostic], code: &str) -> &'a Diagnostic {
    diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == code)
        .unwrap_or_else(|| panic!("missing diagnostic {code}: {diagnostics:#?}"))
}

fn contains_junk(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(contains_junk),
        Value::Object(fields) => {
            fields.get("type").and_then(Value::as_str) == Some("Junk")
                || fields.values().any(contains_junk)
        }
        _ => false,
    }
}

#[test]
fn valid_fluent_has_no_diagnostics() {
    let source = "hello = Hello, { $name }!\n";

    assert!(lint(source).is_empty());
}

#[test]
fn adopted_canonical_valid_files_have_no_diagnostics() {
    const FIXTURES: &[&str] = &[
        "eof_comment.ftl",
        "eof_empty.ftl",
        "eof_value.ftl",
        "literal_expressions.ftl",
        "multiline_values.ftl",
        "term_parameters.ftl",
        "whitespace_in_value.ftl",
        "zero_length.ftl",
    ];

    for name in FIXTURES {
        let source = std::fs::read_to_string(format!(
            "{}/../../test/fixtures/projectfluent-reference/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("fixture must be readable");
        assert!(
            lint(&source).is_empty(),
            "unexpected diagnostics for {name}"
        );
    }
}

#[test]
fn adopted_structure_vectors_keep_their_core_error_codes() {
    const CASES: &[(&str, &[&str])] = &[
        ("message_with_empty_pattern.ftl", &["E0005"]),
        ("attribute_with_empty_pattern.ftl", &["E0012"]),
        ("multiline_string.ftl", &["E0020"]),
        ("select_expressions.ftl", &["E0010"]),
        ("variants_with_two_defaults.ftl", &["E0015"]),
        ("placeable_in_placeable.ftl", &["E0003", "E0027"]),
    ];

    for (name, codes) in CASES {
        let source = std::fs::read_to_string(format!(
            "{}/../../test/fixtures/fluent-tooling-structure/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("fixture must be readable");
        let diagnostics = lint(&source);
        for code in *codes {
            diagnostic(&diagnostics, code);
        }
    }
}

#[test]
fn every_upstream_junk_resource_produces_a_lint_diagnostic() {
    for directory in ["projectfluent-reference", "fluent-tooling-structure"] {
        let fixture_dir = format!(
            "{}/../../test/fixtures/{directory}",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut expectations: Vec<_> = std::fs::read_dir(fixture_dir)
            .expect("fixture directory must exist")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect();
        expectations.sort();

        for expectation_path in expectations {
            let expectation: Value = serde_json::from_str(
                &std::fs::read_to_string(&expectation_path).expect("expectation must be readable"),
            )
            .expect("expectation must be JSON");
            if !contains_junk(&expectation) {
                continue;
            }
            let source_path = expectation_path.with_extension("ftl");
            let source = std::fs::read_to_string(&source_path).expect("fixture must be readable");
            assert!(
                !lint(&source).is_empty(),
                "upstream-invalid fixture {} was silently accepted",
                source_path.display()
            );
        }
    }
}

#[test]
fn every_upstream_non_junk_resource_lints_cleanly() {
    for directory in ["projectfluent-reference", "fluent-tooling-structure"] {
        let fixture_dir = format!(
            "{}/../../test/fixtures/{directory}",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut expectations: Vec<_> = std::fs::read_dir(fixture_dir)
            .expect("fixture directory must exist")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect();
        expectations.sort();

        for expectation_path in expectations {
            let expectation: Value = serde_json::from_str(
                &std::fs::read_to_string(&expectation_path).expect("expectation must be readable"),
            )
            .expect("expectation must be JSON");
            if contains_junk(&expectation) {
                continue;
            }
            let source_path = expectation_path.with_extension("ftl");
            let source = std::fs::read_to_string(&source_path).expect("fixture must be readable");
            let diagnostics = lint(&source);
            assert!(
                diagnostics.is_empty(),
                "upstream-valid fixture {} produced diagnostics: {:#?}",
                source_path.display(),
                diagnostics
            );
        }
    }
}

#[test]
fn broken_fixture_has_actionable_layered_diagnostics() {
    let source = include_str!("../../../test/fixtures/regressions/broken.ftl");
    let diagnostics = lint(source);

    let close_placeable = diagnostic(&diagnostics, "E0003");
    assert_eq!(close_placeable.severity, Severity::Error);
    assert!(close_placeable.message.contains("expected token `}`"));
    assert!(close_placeable.labels.len() >= 2);
    assert!(
        close_placeable
            .help
            .iter()
            .any(|help| help.contains("close the placeable"))
    );
    diagnostic(&diagnostics, "E0010");
    diagnostic(&diagnostics, "E0020");
    diagnostic(&diagnostics, "W1001");

    for diagnostic in diagnostics {
        for label in diagnostic.labels {
            assert!(label.span.start <= label.span.end);
            assert!(label.span.end <= source.len());
            assert!(source.is_char_boundary(label.span.start));
            assert!(source.is_char_boundary(label.span.end));
        }
    }
}

#[test]
fn empty_message_reports_expected_value_or_attributes() {
    let diagnostics = lint("message-id =");

    let diagnostic = diagnostic(&diagnostics, "E0005");
    assert!(diagnostic.message.contains("value or attributes"));
    assert_eq!(diagnostic.labels[0].span, 12..12);
}

#[test]
fn multiline_string_reports_its_opening_and_line_break() {
    let diagnostics = lint("bad = { \"first\nsecond\" }\ngood = Good\n");

    let diagnostic = diagnostic(&diagnostics, "E0020");
    assert!(diagnostic.labels.len() >= 2);
    assert!(
        diagnostic
            .help
            .iter()
            .any(|help| help.contains("single line"))
    );
}

#[test]
fn missing_and_duplicate_default_variants_are_distinguished() {
    let missing = lint("bad = { $value ->\n    [one] One\n}\n");
    let duplicate = lint(concat!(
        "bad = { $value ->\n",
        "   *[one] One\n",
        "   *[other] Other\n",
        "}\n",
    ));

    diagnostic(&missing, "E0010");
    diagnostic(&duplicate, "E0015");
}

#[test]
fn invalid_text_and_entries_receive_specific_diagnostics() {
    diagnostic(&lint("bad = A stray } brace\n"), "E0027");
    diagnostic(&lint("8bad = Value\n"), "E0002");
}

#[test]
fn separate_bad_placeables_produce_separate_primary_spans() {
    let source = "bad = First { $one ? } middle { $two ? }\ngood = Good\n";
    let diagnostics = lint(source);
    let primary_spans: Vec<_> = diagnostics
        .iter()
        .flat_map(|diagnostic| &diagnostic.labels)
        .filter(|label| label.primary)
        .map(|label| label.span.clone())
        .collect();

    assert!(
        primary_spans
            .iter()
            .any(|span| &source[span.clone()] == "?")
    );
    assert!(
        primary_spans
            .iter()
            .filter(|span| &source[(*span).clone()] == "?")
            .count()
            >= 2
    );
}

#[test]
fn unicode_before_an_error_keeps_a_utf8_boundary_span() {
    let source = "message = lubi\u{0105} \u{1f602} { $name ? }\n";
    let diagnostics = lint(source);
    let question = source.find('?').expect("fixture contains question mark");

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .labels
            .iter()
            .any(|label| label.primary && label.span == (question..question + 1))
    }));
}

#[test]
fn plain_renderer_contains_code_location_labels_and_help() {
    let source = include_str!("../../../test/fixtures/regressions/broken.ftl");
    let output = render_plain("broken.ftl", source, &lint(source));

    assert!(output.contains("error[E0003]: expected token `}`"));
    assert!(output.contains("broken.ftl:4:31"));
    assert!(output.contains("close the placeable"));
    assert!(output.contains("help:"));
    assert!(!output.contains('\u{1b}'));
}
