# fluent-tooling

This repository contains [`ftl-lint`](docs/ftl-lint.md), a command-line linter
with compiler-style diagnostics; [`tree-sitter-fluent`](docs/tree-sitter.md),
an error-tolerant Tree-sitter grammar with Rust bindings; and
[`fluent-ts-mode`](docs/emacs.md), a native Tree-sitter major mode for Emacs.

## Quick start

Run the linter from this checkout:

```bash
nix run . -- path/to/messages.ftl
```

See the [`ftl-lint` guide](docs/ftl-lint.md) for installation, command-line
options, diagnostic codes, and editor integration.

## Nix packages

The flake exports two named packages and selects `ftl-lint` by default:

| Command                               | Package                          |
| ------------------------------------- | -------------------------------- |
| `nix build` or `nix build .#ftl-lint` | The `ftl-lint` executable        |
| `nix build .#tree-sitter-fluent`      | The compiled Tree-sitter grammar |

See the [`tree-sitter-fluent` guide](docs/tree-sitter.md) for the grammar
package's layout and integration options.

## Development

Enter the reproducible development shell and run the complete quality gate:

```bash
nix develop
bb ci
```

The [development guide](docs/development.md) covers repository structure,
parser generation, individual checks, and fixture provenance.

## Licensing

Copyright © 2017-present David Rios for the original Tree-sitter grammar.

Copyright © 2026 Casey Link <casey@outskirtslabs.com>

Distributed under the [MIT](https://spdx.org/licenses/MIT.html) license.

Copied grammar provenance and third-party fixture licenses are documented in
[`NOTICE.md`](NOTICE.md) and
[`test/fixtures/UPSTREAM.md`](test/fixtures/UPSTREAM.md).
