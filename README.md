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

### JSON diagnostics protocol

Use `--format json` for editor integration. Human-readable output remains the default. JSON diagnostics go to standard output; invocation and I/O errors remain on standard error. Exit statuses have the same meanings in both formats.

The JSON document has this stable schema:

```json
{
  "schema_version": 1,
  "diagnostics": [
    {
      "path": "locales/pl/messages.ftl",
      "severity": "error",
      "code": "E0003",
      "message": "expected token `}`",
      "labels": [
        {
          "primary": true,
          "message": "unexpected `?` inside the placeable",
          "span": {
            "start": { "byte": 30, "line": 0, "column": 26 },
            "end": { "byte": 31, "line": 0, "column": 27 }
          }
        }
      ],
      "notes": [],
      "help": ["close the placeable before `?`"]
    }
  ]
}
```

`schema_version` is `1`. Consumers should reject unsupported versions. `diagnostics` is always an array, including for clean input and I/O failures. Each diagnostic contains the input path, or `<stdin>` for standard input. `severity` is `error` or `warning`; `code` is the stable Fluent or project lint code.

Every label contains a message, a primary-label flag, and a half-open source span. `byte` is a zero-based UTF-8 byte offset. `line` and `column` are zero-based Unicode scalar-value positions. The end position is exclusive. Empty spans have identical start and end positions. The `labels`, `notes`, and `help` arrays are always present.

## Emacs

The repository includes `fluent-ts-mode`, a native Tree-sitter major mode for Emacs 29.1 and newer. It registers `.ftl` files, provides syntax highlighting and indentation, and supports Flymake and Flycheck through `fl-lint`.

The package does not bundle the compiled grammar. Install the package first, then install the grammar explicitly as described below.

### Vanilla Emacs with package-vc

Emacs 29.1 and newer can install the package directly from its Git repository:

```elisp
(package-vc-install
 '(fluent-ts-mode
   :url "https://github.com/outskirtslabs/fluent-tooling"
   :lisp-dir "editors/emacs"
   :main-file "fluent-ts-mode.el"))

(require 'fluent-ts-mode)
```

### Local checkout

Load the package directly while developing it:

```elisp
(add-to-list 'load-path
             "/absolute/path/to/fluent-tooling/editors/emacs")
(require 'fluent-ts-mode)
```

### straight.el and Doom Emacs

Use this straight.el recipe:

```elisp
(straight-use-package
 '(fluent-ts-mode
   :type git
   :host github
   :repo "outskirtslabs/fluent-tooling"
   :files ("editors/emacs/*.el")))
```

For Doom Emacs, put the equivalent recipe in `packages.el`:

```elisp
(package! fluent-ts-mode
  :recipe (:host github
           :repo "outskirtslabs/fluent-tooling"
           :files ("editors/emacs/*.el")))
```

Run `doom sync`, then restart Emacs. Require the package and configure it in `config.el`.

### Install the grammar

Run `M-x fluent-ts-mode-install-grammar` once after installing the package. The command uses the public recipe for this repository. Opening an `.ftl` file never downloads or installs the grammar.

To build from an existing checkout instead, set the recipe before running the command:

```elisp
(setf (alist-get 'fluent treesit-language-source-alist)
      '("/absolute/path/to/fluent-tooling"))
```

### Configure diagnostics

The default `auto` setting uses Flycheck when it is already loaded; otherwise it uses built-in Flymake. Install `fl-lint` on Emacs's `exec-path`, or set its absolute path:

```elisp
(setq fluent-ts-mode-linter-executable "/absolute/path/to/fl-lint")
```

Select Flymake explicitly without installing another Emacs package:

```elisp
(setq fluent-ts-mode-checker 'flymake)
```

To use Flycheck, install and load Flycheck before opening an `.ftl` buffer:

```elisp
(require 'flycheck)
(setq fluent-ts-mode-checker 'flycheck)
```

Set `fluent-ts-mode-checker` to nil to disable diagnostics.

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
