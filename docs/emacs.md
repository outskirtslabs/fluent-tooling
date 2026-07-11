# Emacs integration

`fluent-ts-mode` is a native Tree-sitter major mode for Fluent FTL resources.
It requires Emacs 29.1 or newer with built-in Tree-sitter support.

The mode registers `.ftl` files and provides syntax highlighting, four-space
indentation, comments, Imenu, defun navigation, and syntax navigation. It can
check unsaved buffer contents with built-in Flymake or optional Flycheck
support through `ftl-lint`.

The Emacs package does not bundle a compiled grammar. Install the package and
then install or expose the grammar explicitly. Opening an `.ftl` file never
downloads or installs software.

## Install the package

### Vanilla Emacs with `package-vc`

Emacs 29.1 and newer can install the package directly from this repository:

```elisp
(package-vc-install
 '(fluent-ts-mode
   :url "https://github.com/outskirtslabs/fluent-tooling"
   :lisp-dir "editors/emacs"
   :main-file "fluent-ts-mode.el"))

(require 'fluent-ts-mode)
```

### straight.el

```elisp
(straight-use-package
 '(fluent-ts-mode
   :type git
   :host github
   :repo "outskirtslabs/fluent-tooling"
   :files ("editors/emacs/*.el")))
```

### Doom Emacs

Add the package recipe to `packages.el`:

```elisp
(package! fluent-ts-mode
  :recipe (:host github
           :repo "outskirtslabs/fluent-tooling"
           :files ("editors/emacs/*.el")))
```

Run `doom sync`, then load and configure the mode from `config.el` or a
language module:

```elisp
(require 'fluent-ts-mode)
(setq fluent-ts-mode-checker 'auto)
```

### Local checkout

Load the package directly while developing it:

```elisp
(add-to-list 'load-path
             "/absolute/path/to/fluent-tooling/editors/emacs")
(require 'fluent-ts-mode)
```

## Install the grammar

Choose one of the following methods. Install the grammar before opening an
FTL file with `fluent-ts-mode`.

### Build from the public source recipe

Run `M-x fluent-ts-mode-install-grammar`. The command asks Emacs's built-in
Tree-sitter installer to clone this repository, compile `src/parser.c` and
`src/scanner.c`, and install the resulting shared library. This command is the
only part of `fluent-ts-mode` that may access the network.

With a prefix argument, the command prompts for an installation directory:

```text
C-u M-x fluent-ts-mode-install-grammar
```

### Build from a local checkout

Replace the public recipe after loading the mode, then run the installation
command:

```elisp
(require 'fluent-ts-mode)
(setf (alist-get 'fluent treesit-language-source-alist)
      '("/absolute/path/to/fluent-tooling"))
```

### Use the Nix grammar package

Build the grammar and add its library directory to Emacs's search path:

```bash
nix build .#tree-sitter-fluent
```

```elisp
(require 'treesit)
(add-to-list 'treesit-extra-load-path
             "/absolute/path/to/fluent-tooling/result/lib")
(require 'fluent-ts-mode)
```

Keep the `result` link available, or use the grammar's immutable Nix store
path. Evaluate `(treesit-language-available-p 'fluent)` to confirm that Emacs
can load it.

## Configure diagnostics

`fluent-ts-mode-checker` controls diagnostics:

| Value      | Behavior                                                      |
| ---------- | ------------------------------------------------------------- |
| `auto`     | Use Flycheck when it is already loaded; otherwise use Flymake |
| `flymake`  | Use built-in Flymake                                          |
| `flycheck` | Require and use Flycheck                                      |
| `nil`      | Disable diagnostics                                           |

The default is `auto`. Load Flycheck before opening an FTL buffer when you
want automatic selection to choose it:

```elisp
(require 'flycheck)
(setq fluent-ts-mode-checker 'auto)
```

Both adapters send unsaved buffer contents to `ftl-lint --format json -`.
Install `ftl-lint` on Emacs's `exec-path`, or set its absolute path:

```elisp
(setq fluent-ts-mode-linter-executable
      "/absolute/path/to/ftl-lint")
```

Select a frontend explicitly when needed:

```elisp
(setq fluent-ts-mode-checker 'flymake)
;; or
(setq fluent-ts-mode-checker 'flycheck)
```

The adapters consume the stable
[`ftl-lint` JSON diagnostics protocol](json-diagnostics.md).

## Customize indentation

Fluent continuation lines use four spaces by default:

```elisp
(setq fluent-ts-mode-indent-offset 4)
```

## Troubleshooting

- If mode activation reports that the grammar is unavailable, install it or
  add its library directory to `treesit-extra-load-path` before opening the
  file.
- If diagnostics report that `ftl-lint` cannot be started, check
  `(executable-find "ftl-lint")` or set
  `fluent-ts-mode-linter-executable` explicitly.
- If `auto` selects Flymake, load Flycheck before activating the FTL buffer or
  set `fluent-ts-mode-checker` to `flycheck`.
