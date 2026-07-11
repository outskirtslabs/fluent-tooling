# Development

The repository provides a reproducible Nix shell with Rust, Tree-sitter,
Babashka, Clang, Emacs, and Flycheck.

## Enter the development shell

```bash
nix develop
```

All commands below assume this shell unless shown with `nix develop --command`.

## Repository layout

| Path                             | Contents                                                     |
| -------------------------------- | ------------------------------------------------------------ |
| `grammar.js`, `src/`, `queries/` | Grammar definition, generated parser, scanner, and queries   |
| `crates/tree-sitter-fluent/`     | Rust grammar bindings and parser tests                       |
| `crates/fluent-lint/`            | Linter library, CLI, and tests                               |
| `editors/emacs/`                 | Major mode, Flymake adapter, Flycheck checker, and ERT tests |
| `test/fixtures/`                 | Upstream and project regression fixtures                     |

## Quality tasks

Babashka exposes the repository's development tasks:

| Command         | Action                                            |
| --------------- | ------------------------------------------------- |
| `bb fmt`        | Format Rust and grammar metadata sources          |
| `bb fmt:check`  | Check formatting without changing files           |
| `bb lint`       | Run Clippy with warnings denied                   |
| `bb test`       | Run all Rust tests                                |
| `bb test:emacs` | Compile the grammar and run all ERT tests         |
| `bb ci`         | Run formatting, Clippy, Rust tests, and ERT tests |

Run the complete quality gate with:

```bash
bb ci
```

From outside the shell, use:

```bash
nix develop --command bb ci
```

## Regenerate the parser

After editing `grammar.js`, regenerate the checked-in Tree-sitter output:

```bash
bb generate
```

Review changes to `src/parser.c`, `src/grammar.json`, and
`src/node-types.json` with the grammar change. Run `bb ci` after generation.

## Run the linter from Cargo

```bash
cargo run -p fluent-lint --bin ftl-lint -- \
  test/fixtures/regressions/broken.ftl
```

## Test fixtures

The test suite adopts Project Fluent, Fluent.js, and fluent-rs fixtures in
addition to project regressions. Imported fixtures retain exact line endings,
trailing whitespace, and missing final newlines because those properties test
parser behavior.

Record the source, revision, license, and local normalization of imported
fixtures in [`test/fixtures/UPSTREAM.md`](../test/fixtures/UPSTREAM.md). Grammar
and fixture license provenance belongs in [`NOTICE.md`](../NOTICE.md).

## Nix validation

Validate both packages and every flake check before release:

```bash
nix build
nix build .#tree-sitter-fluent
nix flake check
```
