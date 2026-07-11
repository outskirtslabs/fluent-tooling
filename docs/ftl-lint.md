# `ftl-lint`

`ftl-lint` checks Fluent FTL resources for syntax and structural problems. It
uses the repository's error-tolerant Tree-sitter grammar and reports source
labels, Fluent error codes, notes, and actionable help.

## Build and run

Build the linter with Nix:

```bash
nix build .#ftl-lint
./result/bin/ftl-lint path/to/messages.ftl
```

Run it without creating a result link:

```bash
nix run .#ftl-lint -- path/to/messages.ftl
```

Build it directly with Cargo from this checkout:

```bash
cargo build --release -p fluent-lint --bin ftl-lint
./target/release/ftl-lint path/to/messages.ftl
```

## Inputs

Pass one or more files as separate arguments:

```bash
ftl-lint locales/en/messages.ftl locales/pl/messages.ftl
```

Use `-` once to read an FTL resource from standard input:

```bash
ftl-lint - < locales/en/messages.ftl
```

## Output

Human-readable diagnostics are the default and go to standard error. Color is
enabled when standard error is a terminal, unless `NO_COLOR` is set. Override
the choice with `--color always`, `--color never`, or `--no-color`.

Use `--format json` for editor and tool integration. JSON diagnostics go to
standard output, while invocation and I/O errors remain on standard error.
The [JSON diagnostics protocol](json-diagnostics.md) defines the stable
integration interface.

## Options

| Option              | Meaning                                                 |
| ------------------- | ------------------------------------------------------- |
| `--format <FORMAT>` | Select `human` or `json` output; defaults to `human`    |
| `--color <WHEN>`    | Select `auto`, `always`, or `never`; defaults to `auto` |
| `--no-color`        | Alias for `--color never`                               |
| `-h`, `--help`      | Print command help                                      |
| `-V`, `--version`   | Print the version                                       |

## Exit status

| Status | Meaning                              |
| ------ | ------------------------------------ |
| `0`    | Every input is clean                 |
| `1`    | At least one diagnostic was reported |
| `2`    | Invocation or I/O failure            |

An I/O failure takes precedence over diagnostics from other inputs, so a
multi-file run returns status 2 if any file cannot be read.

## Diagnostic codes

The linter emits these Project Fluent syntax codes where applicable:

| Code    | Meaning                                |
| ------- | -------------------------------------- |
| `E0002` | Invalid entry or syntax                |
| `E0003` | Expected token                         |
| `E0005` | Message requires a value or attributes |
| `E0006` | Term requires a value                  |
| `E0010` | Missing default variant                |
| `E0012` | Missing attribute value                |
| `E0015` | Duplicate default variant              |
| `E0019` | Term attribute used as a placeable     |
| `E0020` | Unterminated string                    |
| `E0027` | Unbalanced closing brace               |

`W1001` is a project lint that replaces redundant `PLURAL(...)` wrappers with
native Fluent selection.
