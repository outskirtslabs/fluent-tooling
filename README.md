# Fluent Tooling

Fluent Tooling provides an error-tolerant Tree-sitter grammar for Fluent FTL and the `fl-lint` command-line linter.

The parser is designed for editors, linting, incremental parsing, and source-preserving tooling.

The linter reports syntax and structural problems with source labels, Fluent error codes, notes, and actionable help.

## Use `fl-lint`

Build the release executable with Nix:

```bash
nix build
./result/bin/fl-lint path/to/messages.ftl
```

Run it without creating a result link:

```bash
nix run . -- path/to/messages.ftl
```

Pass multiple files as separate arguments or use `-` to read FTL from standard input.

Color is selected automatically for terminals and can be controlled with `--color always`, `--color never`, or `--no-color`.

Exit status 0 means every input is clean, 1 means diagnostics were reported, and 2 means invocation or I/O failed.

The linter currently emits these Project Fluent syntax codes where applicable:

- `E0002`: invalid entry or syntax;
- `E0003`: expected token;
- `E0005`: message requires a value or attributes;
- `E0006`: term requires a value;
- `E0010`: missing default variant;
- `E0012`: missing attribute value;
- `E0015`: duplicate default variant;
- `E0019`: term attribute used as a placeable;
- `E0020`: unterminated string; and
- `E0027`: unbalanced closing brace.

`W1001` is a project lint that replaces redundant `PLURAL(...)` wrappers with native Fluent selection.

## Use the grammar

The `tree-sitter-fluent` workspace crate exposes the generated language and highlight query:

```rust
let mut parser = tree_sitter::Parser::new();
parser.set_language(&tree_sitter_fluent::LANGUAGE.into())?;
```

The CST exposes default variants as `default_variant` nodes, retains malformed regions as Tree-sitter errors, and recovers later valid entries.

The grammar accepts LF, CRLF, and lone CR line endings, negative numeric literals, EOF comments, unindented block placeables, and deeply nested selectors.

The parser and scanner in `src/` are C because that is Tree-sitter's generated parser ABI and the inherited grammar's external scanner.

All new linting and command-line code is Rust.

## Development

Enter the Nix development shell:

```bash
nix develop
```

Run the complete quality gate:

```bash
bb ci
```

Generate the Tree-sitter parser after editing `grammar.js`:

```bash
bb generate
```

Run the linter:

```bash
cargo run -p fluent-lint --bin fl-lint -- test/fixtures/regressions/broken.ftl
```

The test suite adopts Project Fluent, Fluent.js, and fluent-rs fixtures in addition to project regressions.

Imported fixtures retain exact line endings, trailing whitespace, and missing final newlines.

## Licensing

Copyright © 2017-present David Rios for the original Tree-sitter grammar.

Copyright © 2026 Casey Link <casey@outskirtslabs.com>

Distributed under the [MIT](https://spdx.org/licenses/MIT.html).

Copied grammar provenance and third-party fixture licenses are documented in [`NOTICE.md`](NOTICE.md) and [`test/fixtures/UPSTREAM.md`](test/fixtures/UPSTREAM.md).
