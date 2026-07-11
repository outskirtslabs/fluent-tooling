use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use tree_sitter::{InputEdit, Node, Parser, Point, Query, Tree};

fn parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_fluent::LANGUAGE.into())
        .expect("Fluent grammar must load");
    if std::env::var_os("FLUENT_PARSER_DEBUG").is_some() {
        parser.set_logger(Some(Box::new(|kind, message| {
            eprintln!("{kind:?}: {message}");
        })));
    }
    parser
}

fn parse(source: &str) -> Tree {
    parser()
        .parse(source, None)
        .expect("parser returned no tree")
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_nodes<'tree>(node: Node<'tree>, kind: &str, nodes: &mut Vec<Node<'tree>>) {
    if node.kind() == kind {
        nodes.push(node);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nodes(child, kind, nodes);
    }
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

fn assert_upstream_non_junk_fixtures_parse_cleanly(directory: &str) {
    let fixture_dir = repository_root().join(directory);
    let mut expectations: Vec<_> = fs::read_dir(&fixture_dir)
        .expect("fixture directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect();
    expectations.sort();

    let mut parser = parser();
    for expectation_path in expectations {
        let expectation: Value = serde_json::from_str(
            &fs::read_to_string(&expectation_path).expect("expectation must be UTF-8"),
        )
        .expect("expectation must be JSON");
        if contains_junk(&expectation) {
            continue;
        }
        let source_path = expectation_path.with_extension("ftl");
        let source = fs::read_to_string(&source_path).expect("fixture must be UTF-8");
        let tree = parser
            .parse(&source, None)
            .expect("parser returned no tree");
        assert!(
            !tree.root_node().has_error(),
            "upstream-valid fixture {} contains errors: {}",
            source_path.display(),
            tree.root_node().to_sexp()
        );
    }
}

#[test]
fn representative_fluent_parses_cleanly() {
    let source = concat!(
        "### Resource note\n",
        "\n",
        "# Attached comment\n",
        "welcome = Hello, { $user }!\n",
        "    .title = Welcome, { user-name }\n",
        "-brand = Acme\n",
        "    .case = Acme's\n",
        "items =\n",
        "    { $count ->\n",
        "        [one] One item\n",
        "       *[other] { $count } items\n",
        "    }\n",
    );

    assert!(!parse(source).root_node().has_error());
}

#[test]
fn empty_and_eof_comments_parse_cleanly() {
    let source = "#\n# \n##\n## \n###\n### \n# No final newline";

    assert!(!parse(source).root_node().has_error());
}

#[test]
fn negative_numbers_parse_cleanly() {
    let source = concat!(
        "negative = { -3.14 }\n",
        "argument = { NUMBER(-2) }\n",
        "variant = { $value ->\n",
        "   *[-1] Negative\n",
        "}\n",
    );

    assert!(!parse(source).root_node().has_error());
}

#[test]
fn crlf_and_unindented_block_placeables_parse_cleanly() {
    let source = "key =\r\n{\".\"}\r\n    continued\r\n";

    assert!(!parse(source).root_node().has_error());
}

#[test]
fn numeric_named_arguments_after_a_positional_argument_parse_cleanly() {
    let source = concat!(
        "-term = Value\n",
        "use = { -term(\"positional\", narg1: 1, narg2: 2) }\n",
    );

    assert!(!parse(source).root_node().has_error());
}

#[test]
fn canonical_term_parameters_parse_cleanly() {
    let source = include_str!("../../../test/fixtures/projectfluent-reference/term_parameters.ftl");

    assert!(!parse(source).root_node().has_error());
}

#[test]
fn highlight_query_compiles_against_the_grammar() {
    Query::new(
        &tree_sitter_fluent::LANGUAGE.into(),
        tree_sitter_fluent::HIGHLIGHTS_QUERY,
    )
    .expect("highlight query must match the generated node types");
}

#[test]
fn an_incremental_edit_can_repair_an_empty_message() {
    let before = "message-id =";
    let after = "message-id = Value";
    let mut parser = parser();
    let mut tree = parser.parse(before, None).expect("initial parse");
    assert!(tree.root_node().has_error());

    tree.edit(&InputEdit {
        start_byte: before.len(),
        old_end_byte: before.len(),
        new_end_byte: after.len(),
        start_position: Point::new(0, before.len()),
        old_end_position: Point::new(0, before.len()),
        new_end_position: Point::new(0, after.len()),
    });
    let repaired = parser
        .parse(after, Some(&tree))
        .expect("incremental repair parse");

    assert!(!repaired.root_node().has_error());
}

#[test]
fn deeply_nested_selects_do_not_hit_an_arbitrary_scanner_limit() {
    const DEPTH: usize = 32;
    let mut source = String::from("key = ");
    for _ in 0..DEPTH {
        source.push_str("{ $value ->\n   *[other] ");
    }
    source.push_str("End\n");
    for _ in 0..DEPTH {
        source.push_str("}\n");
    }

    assert!(
        !parse(&source).root_node().has_error(),
        "{}",
        parse(&source).root_node().to_sexp()
    );
}

#[test]
fn empty_message_is_not_silently_valid() {
    assert!(parse("message-id =").root_node().has_error());
}

#[test]
fn multiline_string_is_not_silently_valid() {
    let source = "bad = { \"first\nsecond\" }\ngood = Good\n";

    assert!(parse(source).root_node().has_error());
}

#[test]
fn stray_closing_brace_is_not_silently_valid() {
    assert!(parse("bad = A stray } brace\n").root_node().has_error());
}

#[test]
fn named_argument_values_must_be_literals() {
    assert!(
        parse("bad = { FUNC(arg: $variable) }\n")
            .root_node()
            .has_error()
    );
}

#[test]
fn default_variants_are_structurally_distinct() {
    let tree = parse("key = { $value ->\n   *[other] Other\n}\n");
    let mut defaults = Vec::new();
    collect_nodes(tree.root_node(), "default_variant", &mut defaults);

    assert_eq!(defaults.len(), 1);
}

#[test]
fn malformed_entries_do_not_hide_a_later_valid_entry() {
    let source = concat!(
        "bad = First { $one ? } middle { $two ? }\n",
        "good-after = After\n",
    );
    let tree = parse(source);
    let mut messages = Vec::new();
    collect_nodes(tree.root_node(), "message", &mut messages);
    let ids: Vec<_> = messages
        .iter()
        .filter_map(|message| message.child_by_field_name("id"))
        .map(|id| &source[id.byte_range()])
        .collect();

    assert!(tree.root_node().has_error());
    assert!(ids.contains(&"good-after"));
}

#[test]
fn empty_multiline_pattern_at_eof_terminates() {
    let source = include_str!(
        "../../../test/fixtures/fluent-tooling-structure/message_with_empty_multiline_pattern.ftl"
    );
    let tree = parse(source);

    assert_eq!(tree.root_node().byte_range(), 0..source.len());
}

#[test]
fn all_structure_fixtures_terminate_and_cover_the_source() {
    let fixture_dir = repository_root().join("test/fixtures/fluent-tooling-structure");
    let mut fixtures: Vec<_> = fs::read_dir(fixture_dir)
        .expect("fixture directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ftl"))
        .collect();
    fixtures.sort();

    let mut parser = parser();
    for fixture in fixtures {
        let source = fs::read_to_string(&fixture).expect("fixture must be UTF-8");
        let tree = parser
            .parse(&source, None)
            .expect("parser returned no tree");
        assert_eq!(
            tree.root_node().byte_range(),
            0..source.len(),
            "{} did not produce a source-covering root",
            fixture.display()
        );
    }
}

#[test]
fn canonical_files_without_junk_parse_cleanly() {
    const VALID_FIXTURES: &[&str] = &[
        "any_char.ftl",
        "cr_multikey.ftl",
        "cr_multilinevalue.ftl",
        "eof_comment.ftl",
        "eof_empty.ftl",
        "eof_value.ftl",
        "literal_expressions.ftl",
        "multiline_values.ftl",
        "sparse_entries.ftl",
        "term_parameters.ftl",
        "whitespace_in_value.ftl",
        "zero_length.ftl",
    ];

    let fixture_dir = repository_root().join("test/fixtures/projectfluent-reference");
    let mut parser = parser();
    for name in VALID_FIXTURES {
        let path = fixture_dir.join(name);
        let source = fs::read_to_string(&path).expect("fixture must be UTF-8");
        let tree = parser
            .parse(&source, None)
            .expect("parser returned no tree");
        assert!(
            !tree.root_node().has_error(),
            "canonical valid fixture {name} contains parser errors: {}",
            tree.root_node().to_sexp()
        );
    }
}

#[test]
fn every_upstream_resource_without_junk_parses_cleanly() {
    assert_upstream_non_junk_fixtures_parse_cleanly("test/fixtures/projectfluent-reference");
    assert_upstream_non_junk_fixtures_parse_cleanly("test/fixtures/fluent-tooling-structure");
}

#[test]
fn fluent_rs_normalization_inputs_parse_cleanly() {
    let fixture_dir = repository_root().join("test/fixtures/fluent-rs-normalized");
    let mut fixtures: Vec<_> = fs::read_dir(fixture_dir)
        .expect("fixture directory must exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ftl"))
        .collect();
    fixtures.sort();

    let mut parser = parser();
    for fixture in fixtures {
        let source = fs::read_to_string(&fixture).expect("fixture must be UTF-8");
        let tree = parser
            .parse(&source, None)
            .expect("parser returned no tree");
        assert!(
            !tree.root_node().has_error(),
            "normalized fixture {} contains errors: {}",
            fixture.display(),
            tree.root_node().to_sexp()
        );
    }
}
