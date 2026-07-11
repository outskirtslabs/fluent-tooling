use std::cmp::Ordering;
use std::ops::Range;

use codespan_reporting::diagnostic::{
    Diagnostic as CodespanDiagnostic, Label, Severity as CodespanSeverity,
};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::Buffer;
use tree_sitter::{Node, Parser, Tree};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLabel {
    pub span: Range<usize>,
    pub message: String,
    pub primary: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub labels: Vec<SourceLabel>,
    pub notes: Vec<String>,
    pub help: Vec<String>,
}

#[derive(Clone, Copy)]
struct Line {
    start: usize,
    content_end: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct EntryHeader {
    line: usize,
    start: usize,
    value_start: usize,
    term: bool,
}

#[derive(Clone, Copy)]
struct AttributeHeader {
    line: usize,
    indent: usize,
    value_start: usize,
}

#[derive(Clone)]
struct StringSpan {
    span: Range<usize>,
    opening: usize,
    first_line_break: Option<usize>,
}

impl StringSpan {
    fn multiline(&self) -> bool {
        self.first_line_break.is_some()
    }
}

#[derive(Clone, Copy)]
struct PlaceableFrame {
    opening: usize,
    parentheses: usize,
    selector: bool,
}

pub fn lint(source: &str) -> Vec<Diagnostic> {
    let lines = source_lines(source);
    let comments = comment_ranges(source, &lines);
    let strings = string_spans(source, &comments);
    let headers = entry_headers(source, &lines);
    let attributes = attribute_headers(source, &lines);
    let tree = parse(source);
    let mut diagnostics = Vec::new();

    analyze_empty_entries(source, &lines, &headers, &mut diagnostics);
    analyze_empty_attributes(source, &lines, &headers, &attributes, &mut diagnostics);
    analyze_invalid_entry_starts(source, &lines, &headers, &mut diagnostics);
    analyze_strings(source, &strings, &mut diagnostics);
    analyze_placeables(source, &comments, &strings, &headers, &mut diagnostics);
    analyze_selectors(source, &comments, &strings, &mut diagnostics);
    analyze_plural_calls(source, &comments, &strings, &mut diagnostics);
    analyze_term_attribute_placeables(tree.root_node(), &mut diagnostics);
    add_tree_sitter_fallbacks(source, &tree, &headers, &mut diagnostics);

    diagnostics.sort_by(compare_diagnostics);
    diagnostics.dedup_by(|left, right| {
        left.code == right.code && primary_span(left) == primary_span(right)
    });
    diagnostics
}

fn parse(source: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_fluent::LANGUAGE.into())
        .expect("the bundled Fluent grammar must load");
    parser
        .parse(source, None)
        .expect("parsing an in-memory Fluent source must return a tree")
}

pub fn render_plain(filename: &str, source: &str, diagnostics: &[Diagnostic]) -> String {
    render(filename, source, diagnostics, false)
}

pub fn render_ansi(filename: &str, source: &str, diagnostics: &[Diagnostic]) -> String {
    render(filename, source, diagnostics, true)
}

fn render(filename: &str, source: &str, diagnostics: &[Diagnostic], ansi: bool) -> String {
    let file = SimpleFile::new(filename, source);
    let config = term::Config::default();
    let mut buffer = if ansi {
        Buffer::ansi()
    } else {
        Buffer::no_color()
    };

    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            Severity::Error => CodespanSeverity::Error,
            Severity::Warning => CodespanSeverity::Warning,
        };
        let labels = diagnostic
            .labels
            .iter()
            .map(|label| {
                let rendered = if label.primary {
                    Label::primary((), label.span.clone())
                } else {
                    Label::secondary((), label.span.clone())
                };
                rendered.with_message(label.message.clone())
            })
            .collect();
        let notes = diagnostic
            .notes
            .iter()
            .map(|note| format!("note: {note}"))
            .chain(diagnostic.help.iter().map(|help| format!("help: {help}")))
            .collect();
        let rendered = CodespanDiagnostic::new(severity)
            .with_code(diagnostic.code.clone())
            .with_message(diagnostic.message.clone())
            .with_labels(labels)
            .with_notes(notes);

        term::emit(&mut buffer, &config, &file, &rendered)
            .expect("writing diagnostics to an in-memory buffer cannot fail");
    }

    String::from_utf8_lossy(buffer.as_slice()).into_owned()
}

fn source_lines(source: &str) -> Vec<Line> {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;

    while start < bytes.len() {
        let mut content_end = start;
        while content_end < bytes.len() && !matches!(bytes[content_end], b'\r' | b'\n') {
            content_end += 1;
        }
        let mut end = content_end;
        if end < bytes.len() {
            if bytes[end] == b'\r' && bytes.get(end + 1) == Some(&b'\n') {
                end += 2;
            } else {
                end += 1;
            }
        }
        lines.push(Line {
            start,
            content_end,
            end,
        });
        start = end;
    }

    lines
}

fn comment_ranges(source: &str, lines: &[Line]) -> Vec<Range<usize>> {
    let bytes = source.as_bytes();
    lines
        .iter()
        .filter(|line| bytes.get(line.start) == Some(&b'#'))
        .map(|line| line.start..line.end)
        .collect()
}

fn entry_headers(source: &str, lines: &[Line]) -> Vec<EntryHeader> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(line_index, line)| parse_entry_header(source, line_index, *line))
        .collect()
}

fn attribute_headers(source: &str, lines: &[Line]) -> Vec<AttributeHeader> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(line_index, line)| parse_attribute_header(source, line_index, *line))
        .collect()
}

fn parse_entry_header(source: &str, line_index: usize, line: Line) -> Option<EntryHeader> {
    let bytes = source.as_bytes();
    let mut cursor = line.start;
    let term = bytes.get(cursor) == Some(&b'-');
    if term {
        cursor += 1;
    }
    if cursor >= line.content_end || !bytes[cursor].is_ascii_alphabetic() {
        return None;
    }
    cursor += 1;
    while cursor < line.content_end && is_identifier_continue(bytes[cursor]) {
        cursor += 1;
    }
    while cursor < line.content_end && bytes[cursor] == b' ' {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'=') {
        return None;
    }
    cursor += 1;
    while cursor < line.content_end && bytes[cursor] == b' ' {
        cursor += 1;
    }

    Some(EntryHeader {
        line: line_index,
        start: line.start,
        value_start: cursor,
        term,
    })
}

fn parse_attribute_header(source: &str, line_index: usize, line: Line) -> Option<AttributeHeader> {
    let bytes = source.as_bytes();
    let mut cursor = line.start;
    while cursor < line.content_end && bytes[cursor] == b' ' {
        cursor += 1;
    }
    let indent = cursor - line.start;
    if bytes.get(cursor) != Some(&b'.') {
        return None;
    }
    cursor += 1;
    if cursor >= line.content_end || !bytes[cursor].is_ascii_alphabetic() {
        return None;
    }
    cursor += 1;
    while cursor < line.content_end && is_identifier_continue(bytes[cursor]) {
        cursor += 1;
    }
    while cursor < line.content_end && bytes[cursor] == b' ' {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'=') {
        return None;
    }
    cursor += 1;
    while cursor < line.content_end && bytes[cursor] == b' ' {
        cursor += 1;
    }
    Some(AttributeHeader {
        line: line_index,
        indent,
        value_start: cursor,
    })
}

fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn analyze_empty_entries(
    source: &str,
    lines: &[Line],
    headers: &[EntryHeader],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bytes = source.as_bytes();
    for header in headers {
        let line = lines[header.line];
        if bytes[header.value_start..line.content_end]
            .iter()
            .any(|byte| *byte != b' ')
        {
            continue;
        }

        let mut has_value = false;
        let mut has_attribute = false;
        for candidate in &lines[header.line + 1..] {
            if parse_entry_header(source, 0, *candidate).is_some()
                || bytes.get(candidate.start) == Some(&b'#')
            {
                break;
            }

            let mut cursor = candidate.start;
            while cursor < candidate.content_end && bytes[cursor] == b' ' {
                cursor += 1;
            }
            if cursor == candidate.content_end {
                continue;
            }
            if bytes[cursor] == b'.' {
                has_attribute = true;
                break;
            }
            if cursor > candidate.start {
                has_value = true;
                break;
            }
            if bytes[cursor] == b'{' {
                has_value = true;
            }
            break;
        }

        if has_value || (!header.term && has_attribute) {
            continue;
        }

        let (code, message, help) = if header.term {
            (
                "E0006",
                "expected a term value",
                "add a value after `=`; terms cannot contain only attributes",
            )
        } else {
            (
                "E0005",
                "expected a message value or attributes",
                "add a value after `=` or define an indented attribute",
            )
        };
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            labels: vec![SourceLabel {
                span: header.value_start..header.value_start,
                message: "this entry is empty".into(),
                primary: true,
            }],
            notes: Vec::new(),
            help: vec![help.into()],
        });
    }
}

fn analyze_empty_attributes(
    source: &str,
    lines: &[Line],
    entries: &[EntryHeader],
    attributes: &[AttributeHeader],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bytes = source.as_bytes();
    for attribute in attributes {
        let line = lines[attribute.line];
        if bytes[attribute.value_start..line.content_end]
            .iter()
            .any(|byte| *byte != b' ')
        {
            continue;
        }

        let mut has_value = false;
        for (candidate_index, candidate) in lines.iter().enumerate().skip(attribute.line + 1) {
            if entries.iter().any(|entry| entry.start == candidate.start)
                || attributes.iter().any(|following| {
                    following.line > attribute.line && following.line == candidate_index
                })
            {
                break;
            }
            let mut cursor = candidate.start;
            while cursor < candidate.content_end && bytes[cursor] == b' ' {
                cursor += 1;
            }
            if cursor == candidate.content_end {
                continue;
            }
            if cursor - candidate.start > attribute.indent {
                has_value = true;
            }
            break;
        }
        if has_value {
            continue;
        }

        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E0012".into(),
            message: "expected an attribute value".into(),
            labels: vec![SourceLabel {
                span: attribute.value_start..attribute.value_start,
                message: "this attribute is empty".into(),
                primary: true,
            }],
            notes: Vec::new(),
            help: vec!["add a value after `=` or on a more deeply indented line".into()],
        });
    }
}

fn analyze_invalid_entry_starts(
    source: &str,
    lines: &[Line],
    headers: &[EntryHeader],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bytes = source.as_bytes();
    let valid_starts: Vec<_> = headers.iter().map(|header| header.start).collect();

    for line in lines {
        if line.start == line.content_end
            || valid_starts.binary_search(&line.start).is_ok()
            || matches!(
                bytes[line.start],
                b' ' | b'\t' | b'#' | b'.' | b'{' | b'}' | b'[' | b'*'
            )
            || bytes[line.start].is_ascii_alphabetic()
            || bytes[line.start] == b'-'
        {
            continue;
        }

        let mut end = line.start + 1;
        while end < line.content_end && !bytes[end].is_ascii_whitespace() && bytes[end] != b'=' {
            end += 1;
        }
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E0002".into(),
            message: "invalid Fluent entry start".into(),
            labels: vec![SourceLabel {
                span: line.start..end,
                message: "an entry must start with a letter, `-`, or `#`".into(),
                primary: true,
            }],
            notes: Vec::new(),
            help: vec!["rename the entry so its identifier starts with an ASCII letter".into()],
        });
    }
}

fn string_spans(source: &str, comments: &[Range<usize>]) -> Vec<StringSpan> {
    let bytes = source.as_bytes();
    let mut spans = Vec::new();
    let mut braces = 0usize;
    let mut cursor = 0;

    while cursor < bytes.len() {
        if let Some(end) = containing_range_end(cursor, comments) {
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            b'{' => braces += 1,
            b'}' => braces = braces.saturating_sub(1),
            b'"' if braces > 0 => {
                let opening = cursor;
                let mut first_line_break = None;
                cursor += 1;
                while cursor < bytes.len() {
                    match bytes[cursor] {
                        b'\\' => cursor = (cursor + 2).min(bytes.len()),
                        b'"' => {
                            cursor += 1;
                            break;
                        }
                        b'\r' | b'\n' => {
                            first_line_break.get_or_insert(cursor);
                            cursor += 1;
                        }
                        _ => cursor += 1,
                    }
                }
                spans.push(StringSpan {
                    span: opening..cursor,
                    opening,
                    first_line_break,
                });
                continue;
            }
            _ => {}
        }
        cursor += 1;
    }

    spans
}

fn analyze_strings(source: &str, strings: &[StringSpan], diagnostics: &mut Vec<Diagnostic>) {
    for string in strings.iter().filter(|string| string.multiline()) {
        let line_break = string
            .first_line_break
            .expect("multiline strings have a line break");
        let break_end = if source.as_bytes().get(line_break) == Some(&b'\r')
            && source.as_bytes().get(line_break + 1) == Some(&b'\n')
        {
            line_break + 2
        } else {
            line_break + 1
        };
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E0020".into(),
            message: "unterminated string literal".into(),
            labels: vec![
                SourceLabel {
                    span: line_break..break_end,
                    message: "the line ends before this string is closed".into(),
                    primary: true,
                },
                SourceLabel {
                    span: string.opening..string.opening + 1,
                    message: "string starts here".into(),
                    primary: false,
                },
            ],
            notes: vec!["Fluent string literals cannot contain a line break".into()],
            help: vec!["keep the string literal on a single line".into()],
        });
    }
}

fn analyze_placeables(
    source: &str,
    comments: &[Range<usize>],
    strings: &[StringSpan],
    headers: &[EntryHeader],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bytes = source.as_bytes();
    let ignored_strings: Vec<_> = strings.iter().map(|string| string.span.clone()).collect();
    let entry_starts: Vec<_> = headers.iter().map(|header| header.start).collect();
    let mut next_entry = 0usize;
    let mut frames: Vec<PlaceableFrame> = Vec::new();
    let mut cursor = 0;

    while cursor < bytes.len() {
        if next_entry < entry_starts.len() && cursor == entry_starts[next_entry] {
            if frames.last().is_some_and(|frame| frame.opening < cursor) {
                emit_unclosed_placeables(cursor, &frames, diagnostics);
                frames.clear();
            }
            next_entry += 1;
        }
        if let Some(end) = containing_range_end(cursor, comments) {
            cursor = end;
            continue;
        }
        if let Some(end) = containing_range_end(cursor, &ignored_strings) {
            cursor = end;
            continue;
        }

        match bytes[cursor] {
            b'{' => frames.push(PlaceableFrame {
                opening: cursor,
                parentheses: 0,
                selector: false,
            }),
            b'}' => close_placeable(cursor, &mut frames, diagnostics),
            b'(' => {
                if let Some(frame) = frames.last_mut() {
                    frame.parentheses += 1;
                }
            }
            b')' => {
                if let Some(frame) = frames.last_mut() {
                    frame.parentheses = frame.parentheses.saturating_sub(1);
                }
            }
            b'-' if bytes.get(cursor + 1) == Some(&b'>') => {
                if let Some(frame) = frames.last_mut() {
                    frame.selector = true;
                }
                cursor += 1;
            }
            b',' | b'?' => diagnose_unexpected_placeable_token(
                cursor,
                bytes[cursor] as char,
                frames.last(),
                diagnostics,
            ),
            _ => {}
        }
        cursor += 1;
    }

    emit_unclosed_placeables(source.len(), &frames, diagnostics);
}

fn diagnose_unexpected_placeable_token(
    cursor: usize,
    token: char,
    frame: Option<&PlaceableFrame>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(frame) = frame else {
        return;
    };
    if frame.selector || frame.parentheses > 0 {
        return;
    }
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        code: "E0003".into(),
        message: "expected token `}`".into(),
        labels: vec![
            SourceLabel {
                span: cursor..cursor + 1,
                message: format!("unexpected `{token}` inside the placeable"),
                primary: true,
            },
            SourceLabel {
                span: frame.opening..frame.opening + 1,
                message: "placeable starts here".into(),
                primary: false,
            },
        ],
        notes: Vec::new(),
        help: vec![format!("close the placeable before `{token}`")],
    });
}

fn close_placeable(
    cursor: usize,
    frames: &mut Vec<PlaceableFrame>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if frames.pop().is_some() {
        return;
    }
    diagnostics.push(Diagnostic {
        severity: Severity::Error,
        code: "E0027".into(),
        message: "unbalanced closing brace in text".into(),
        labels: vec![SourceLabel {
            span: cursor..cursor + 1,
            message: "this brace does not close a placeable".into(),
            primary: true,
        }],
        notes: Vec::new(),
        help: vec!["use `{ \"}\" }` when the output needs a literal closing brace".into()],
    });
}

fn emit_unclosed_placeables(
    point: usize,
    frames: &[PlaceableFrame],
    diagnostics: &mut Vec<Diagnostic>,
) {
    for frame in frames {
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E0003".into(),
            message: "expected token `}`".into(),
            labels: vec![
                SourceLabel {
                    span: point..point,
                    message: "expected the placeable to end before this point".into(),
                    primary: true,
                },
                SourceLabel {
                    span: frame.opening..frame.opening + 1,
                    message: "unclosed placeable starts here".into(),
                    primary: false,
                },
            ],
            notes: Vec::new(),
            help: vec!["close the placeable with `}`".into()],
        });
    }
}

fn analyze_selectors(
    source: &str,
    comments: &[Range<usize>],
    strings: &[StringSpan],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let bytes = source.as_bytes();
    let mut ignored = comments.to_vec();
    ignored.extend(
        strings
            .iter()
            .filter(|string| !string.multiline())
            .map(|string| string.span.clone()),
    );
    ignored.sort_by_key(|range| range.start);

    let mut arrows = Vec::new();
    let mut depth = 0usize;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if let Some(end) = containing_range_end(cursor, &ignored) {
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'-' if bytes.get(cursor + 1) == Some(&b'>') && depth > 0 => {
                arrows.push((cursor, depth));
                cursor += 1;
            }
            _ => {}
        }
        cursor += 1;
    }

    for (arrow, base_depth) in arrows {
        let defaults = selector_defaults(bytes, arrow + 2, base_depth, &ignored);
        if defaults.is_empty() {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                code: "E0010".into(),
                message: "select expression requires a default variant".into(),
                labels: vec![SourceLabel {
                    span: arrow..arrow + 2,
                    message: "no default variant follows this selector".into(),
                    primary: true,
                }],
                notes: vec!["the default is used when no other variant key matches".into()],
                help: vec!["mark one variant as default by placing `*` directly before `[`".into()],
            });
        } else {
            for duplicate in &defaults[1..] {
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    code: "E0015".into(),
                    message: "select expression has more than one default variant".into(),
                    labels: vec![
                        SourceLabel {
                            span: *duplicate..*duplicate + 1,
                            message: "additional default variant".into(),
                            primary: true,
                        },
                        SourceLabel {
                            span: defaults[0]..defaults[0] + 1,
                            message: "first default variant".into(),
                            primary: false,
                        },
                    ],
                    notes: Vec::new(),
                    help: vec!["remove `*` from every variant except the intended fallback".into()],
                });
            }
        }
    }
}

fn selector_defaults(
    bytes: &[u8],
    start: usize,
    base_depth: usize,
    ignored: &[Range<usize>],
) -> Vec<usize> {
    let mut defaults = Vec::new();
    let mut depth = base_depth;
    let mut cursor = start;
    let mut line_prefix = false;

    while cursor < bytes.len() {
        if let Some(end) = containing_range_end(cursor, ignored) {
            if bytes[cursor..end]
                .iter()
                .any(|byte| matches!(byte, b'\r' | b'\n'))
            {
                line_prefix = true;
            }
            cursor = end;
            continue;
        }
        match bytes[cursor] {
            b'\r' | b'\n' => line_prefix = true,
            b' ' if line_prefix => {}
            b'*' if line_prefix && depth == base_depth => {
                if bytes.get(cursor + 1) == Some(&b'[') {
                    defaults.push(cursor);
                }
                line_prefix = false;
            }
            b'{' => {
                line_prefix = false;
                depth += 1;
            }
            b'}' => {
                line_prefix = false;
                depth = depth.saturating_sub(1);
                if depth < base_depth {
                    break;
                }
            }
            _ => line_prefix = false,
        }
        cursor += 1;
    }

    defaults
}

fn analyze_plural_calls(
    source: &str,
    comments: &[Range<usize>],
    strings: &[StringSpan],
    diagnostics: &mut Vec<Diagnostic>,
) {
    const PLURAL: &[u8] = b"PLURAL";
    let bytes = source.as_bytes();
    let mut ignored = comments.to_vec();
    ignored.extend(
        strings
            .iter()
            .filter(|string| !string.multiline())
            .map(|string| string.span.clone()),
    );
    ignored.sort_by_key(|range| range.start);

    let mut cursor = 0;
    while cursor + PLURAL.len() <= bytes.len() {
        if let Some(end) = containing_range_end(cursor, &ignored) {
            cursor = end;
            continue;
        }
        if &bytes[cursor..cursor + PLURAL.len()] != PLURAL
            || cursor
                .checked_sub(1)
                .and_then(|index| bytes.get(index))
                .is_some_and(|byte| is_identifier_continue(*byte))
            || bytes
                .get(cursor + PLURAL.len())
                .is_some_and(|byte| is_identifier_continue(*byte))
        {
            cursor += 1;
            continue;
        }

        let mut open = cursor + PLURAL.len();
        while bytes.get(open) == Some(&b' ') {
            open += 1;
        }
        if bytes.get(open) != Some(&b'(') {
            cursor += PLURAL.len();
            continue;
        }
        let Some(close) = matching_parenthesis(bytes, open) else {
            cursor += PLURAL.len();
            continue;
        };
        let mut arrow = close + 1;
        while bytes
            .get(arrow)
            .is_some_and(|byte| matches!(byte, b' ' | b'\r' | b'\n'))
        {
            arrow += 1;
        }
        if bytes.get(arrow..arrow + 2) != Some(b"->") {
            cursor += PLURAL.len();
            continue;
        }

        diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            code: "W1001".into(),
            message: "explicit `PLURAL` call is unnecessary".into(),
            labels: vec![SourceLabel {
                span: cursor..cursor + PLURAL.len(),
                message: "Fluent selectors apply plural rules directly".into(),
                primary: true,
            }],
            notes: vec!["a numeric selector already chooses CLDR plural categories".into()],
            help: vec![
                "select on the variable or expression without wrapping it in `PLURAL`".into(),
            ],
        });
        cursor += PLURAL.len();
    }
}

fn matching_parenthesis(bytes: &[u8], opening: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, byte) in bytes[opening..].iter().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(opening + offset);
                }
            }
            b'\r' | b'\n' if depth == 1 => return None,
            _ => {}
        }
    }
    None
}

fn analyze_term_attribute_placeables(node: Node<'_>, diagnostics: &mut Vec<Diagnostic>) {
    if node.kind() == "term_reference"
        && node.child_by_field_name("attribute").is_some()
        && node.parent().map(|parent| parent.kind()) != Some("selector_expression")
    {
        let attribute = node
            .child_by_field_name("attribute")
            .expect("attribute presence was checked");
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E0019".into(),
            message: "attributes of terms cannot be used as placeables".into(),
            labels: vec![SourceLabel {
                span: attribute.byte_range(),
                message: "term attribute is used as an inline value".into(),
                primary: true,
            }],
            notes: vec!["term attributes may be used as select-expression selectors".into()],
            help: vec!["reference the term itself or move this reference into a selector".into()],
        });
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        analyze_term_attribute_placeables(child, diagnostics);
    }
}

fn add_tree_sitter_fallbacks(
    source: &str,
    tree: &Tree,
    headers: &[EntryHeader],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !tree.root_node().has_error() {
        return;
    }

    let mut missing = Vec::new();
    let mut errors = Vec::new();
    collect_problem_nodes(tree.root_node(), &mut missing, &mut errors);

    for node in missing {
        let span = node.byte_range();
        if explained(&span, diagnostics) {
            continue;
        }
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E0003".into(),
            message: format!("expected token `{}`", node.kind()),
            labels: vec![SourceLabel {
                span,
                message: format!("expected `{}` here", node.kind()),
                primary: true,
            }],
            notes: Vec::new(),
            help: vec!["complete the surrounding Fluent expression".into()],
        });
    }

    errors.sort_by_key(|node| node.end_byte() - node.start_byte());
    for node in errors.into_iter().take(12) {
        let span = node.byte_range();
        if explained(&span, diagnostics)
            || entry_has_specific_diagnostic(&span, source.len(), headers, diagnostics)
        {
            continue;
        }
        diagnostics.push(Diagnostic {
            severity: Severity::Error,
            code: "E0002".into(),
            message: "invalid Fluent syntax".into(),
            labels: vec![SourceLabel {
                span,
                message: "Tree-sitter recovered from this input".into(),
                primary: true,
            }],
            notes: Vec::new(),
            help: vec!["check the surrounding entry, placeable, and selector syntax".into()],
        });
    }
}

fn entry_has_specific_diagnostic(
    span: &Range<usize>,
    source_len: usize,
    headers: &[EntryHeader],
    diagnostics: &[Diagnostic],
) -> bool {
    let position = span.start.min(source_len);
    let Some(index) = headers.iter().rposition(|header| header.start <= position) else {
        return false;
    };
    let entry = headers[index].start
        ..headers
            .get(index + 1)
            .map_or(source_len, |header| header.start);

    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code != "E0002")
        .filter_map(primary_span)
        .any(|primary| entry.start <= primary.start && primary.start < entry.end)
}

fn collect_problem_nodes<'tree>(
    node: Node<'tree>,
    missing: &mut Vec<Node<'tree>>,
    errors: &mut Vec<Node<'tree>>,
) {
    if node.is_missing() {
        missing.push(node);
    }
    if node.is_error() {
        errors.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_problem_nodes(child, missing, errors);
    }
}

fn containing_range_end(index: usize, ranges: &[Range<usize>]) -> Option<usize> {
    ranges
        .iter()
        .find(|range| range.start <= index && index < range.end)
        .map(|range| range.end)
}

fn explained(span: &Range<usize>, diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .flat_map(|diagnostic| &diagnostic.labels)
        .any(|label| {
            if span.is_empty() {
                label.span.start <= span.start && span.start <= label.span.end
            } else if label.span.is_empty() {
                span.start <= label.span.start && label.span.start <= span.end
            } else {
                span.start < label.span.end && label.span.start < span.end
            }
        })
}

fn primary_span(diagnostic: &Diagnostic) -> Option<Range<usize>> {
    diagnostic
        .labels
        .iter()
        .find(|label| label.primary)
        .map(|label| label.span.clone())
}

fn compare_diagnostics(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    let left_start = primary_span(left).map_or(usize::MAX, |span| span.start);
    let right_start = primary_span(right).map_or(usize::MAX, |span| span.start);
    left_start
        .cmp(&right_start)
        .then_with(|| match (left.severity, right.severity) {
            (Severity::Error, Severity::Warning) => Ordering::Less,
            (Severity::Warning, Severity::Error) => Ordering::Greater,
            _ => Ordering::Equal,
        })
        .then_with(|| left.code.cmp(&right.code))
}
