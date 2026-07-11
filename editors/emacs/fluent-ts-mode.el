;;; fluent-ts-mode.el --- Tree-sitter mode for Fluent FTL  -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Casey Link

;; Author: Casey Link <casey@outskirtslabs.com>
;; Maintainer: Casey Link <casey@outskirtslabs.com>
;; Version: 0.1.0
;; Package-Requires: ((emacs "29.1"))
;; Keywords: languages, i18n, fluent, tree-sitter
;; URL: https://github.com/outskirtslabs/fluent-tooling
;; SPDX-License-Identifier: MIT

;;; Commentary:

;; `fluent-ts-mode' edits Fluent Translation List (FTL) resources with
;; Emacs's built-in Tree-sitter support.  Install the grammar explicitly with
;; `fluent-ts-mode-install-grammar' before opening an FTL file.  Activating the
;; mode never downloads or installs software.

;;; Code:

(when (version< emacs-version "29.1")
  (error "fluent-ts-mode requires Emacs 29.1 or newer"))

(require 'treesit)
(eval-when-compile (require 'rx))

(declare-function treesit-install-language-grammar "treesit.el")
(declare-function treesit-node-child-by-field-name "treesit.c")
(declare-function treesit-node-parent "treesit.c")
(declare-function treesit-node-start "treesit.c")
(declare-function treesit-node-text "treesit.c")
(declare-function treesit-node-type "treesit.c")
(declare-function treesit-parser-create "treesit.c")
(declare-function flycheck-mode "ext:flycheck")
(declare-function fluent-ts-flymake-backend "fluent-ts-flymake")

(defvar flycheck-checker)

(defgroup fluent-ts nil
  "Editing Fluent FTL resources with Tree-sitter."
  :group 'languages
  :prefix "fluent-ts-mode-")

(defcustom fluent-ts-mode-indent-offset 4
  "Number of spaces in one Fluent indentation level."
  :type 'natnum
  :safe #'natnump
  :group 'fluent-ts)

(defcustom fluent-ts-mode-linter-executable "fl-lint"
  "Executable used to lint Fluent buffers."
  :type 'string
  :safe #'stringp
  :group 'fluent-ts)

(defcustom fluent-ts-mode-checker 'auto
  "Diagnostic frontend enabled by `fluent-ts-mode'.

`auto' uses Flycheck when it is already loaded in the current Emacs session
and otherwise uses built-in Flymake.  `flycheck' and `flymake' select a
frontend explicitly.  A nil value disables lint integration."
  :type '(choice (const :tag "Auto-detect" auto)
                 (const :tag "Flycheck" flycheck)
                 (const :tag "Flymake" flymake)
                 (const :tag "Disabled" nil))
  :safe (lambda (value) (memq value '(auto flycheck flymake nil)))
  :group 'fluent-ts)

(defconst fluent-ts-mode--grammar-url
  "https://github.com/outskirtslabs/fluent-tooling"
  "Public source repository for the Fluent Tree-sitter grammar.")

(add-to-list 'treesit-language-source-alist
             `(fluent . (,fluent-ts-mode--grammar-url)))

(defvar fluent-ts-mode--syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?# "<" table)
    (modify-syntax-entry ?\n ">" table)
    (modify-syntax-entry ?\^m "> b" table)
    (modify-syntax-entry ?_ "w" table)
    (modify-syntax-entry ?- "w" table)
    (modify-syntax-entry ?$ "'" table)
    (modify-syntax-entry ?\\ "\\" table)
    (modify-syntax-entry ?\" "\"" table)
    (modify-syntax-entry ?{ "(}" table)
    (modify-syntax-entry ?} "){" table)
    (modify-syntax-entry ?\[ "(]" table)
    (modify-syntax-entry ?\] ")[" table)
    (modify-syntax-entry ?\( "()" table)
    (modify-syntax-entry ?\) ")(" table)
    table)
  "Syntax table for `fluent-ts-mode'.")

(defvar fluent-ts-mode--font-lock-settings
  (treesit-font-lock-rules
   :language 'fluent
   :feature 'comment
   '([(comment_block)
      (doc_comment_block)
      (file_comment)
      (group_comment)] @font-lock-comment-face)

   :language 'fluent
   :feature 'message
   '((message id: (identifier) @font-lock-variable-name-face)
     (message_reference id: (identifier) @font-lock-variable-use-face))

   :language 'fluent
   :feature 'term
   '((term id: (term_identifier) @font-lock-constant-face)
     (term_reference id: (term_identifier) @font-lock-constant-face))

   :language 'fluent
   :feature 'attribute
   :override t
   '((attribute id: (identifier) @font-lock-property-name-face)
     (message_reference
      attribute: (identifier) @font-lock-property-use-face)
     (term_reference
      attribute: (identifier) @font-lock-property-use-face))

   :language 'fluent
   :feature 'variable
   '((variable) @font-lock-variable-use-face
     (named_argument id: (identifier) @font-lock-variable-name-face))

   :language 'fluent
   :feature 'selector
   '(("->" @font-lock-keyword-face))

   :language 'fluent
   :feature 'variant
   '((selector_variant key: (identifier) @font-lock-type-face)
     (selector_variant key: (number_literal) @font-lock-type-face)
     (default_variant key: (identifier) @font-lock-type-face)
     (default_variant key: (number_literal) @font-lock-type-face)
     (default_variant "*" @font-lock-keyword-face))

   :language 'fluent
   :feature 'function
   '((function_reference
      id: (function_name) @font-lock-function-call-face))

   :language 'fluent
   :feature 'literal
   '((number_literal) @font-lock-number-face
     (string_literal) @font-lock-string-face)

   :language 'fluent
   :feature 'escape-sequence
   :override t
   '((escaped_literal) @font-lock-escape-face)

   :language 'fluent
   :feature 'bracket
   '((["{" "}" "[" "]" "(" ")"]) @font-lock-bracket-face)

   :language 'fluent
   :feature 'operator
   '((["=" "->" ":"]) @font-lock-operator-face)

   :language 'fluent
   :feature 'delimiter
   '((["." ","]) @font-lock-delimiter-face)

   :language 'fluent
   :feature 'error
   :override t
   '([(ERROR) (unfinished_line)] @font-lock-warning-face))
  "Tree-sitter font-lock settings for `fluent-ts-mode'.")

(defconst fluent-ts-mode--defun-regexp
  (rx bos (or "message" "term") eos)
  "Tree-sitter node types treated as Fluent definitions.")

(defun fluent-ts-mode--ancestor (node types)
  "Return NODE's nearest ancestor whose type belongs to TYPES.

NODE itself may match."
  (while (and node (not (member (treesit-node-type node) types)))
    (setq node (treesit-node-parent node)))
  node)

(defun fluent-ts-mode--ancestor-start (node types)
  "Return the start of NODE's nearest ancestor in TYPES."
  (when-let ((ancestor (fluent-ts-mode--ancestor node types)))
    (treesit-node-start ancestor)))

(defun fluent-ts-mode--placeable-for-selector (node)
  "Return the placeable containing NODE's nearest selector."
  (when-let* ((selectors (fluent-ts-mode--ancestor node '("selectors")))
              (placeable (treesit-node-parent selectors)))
    placeable))

(defun fluent-ts-mode--value-anchor (node)
  "Return an indentation anchor and offset for value content at NODE."
  (cond
   ((fluent-ts-mode--ancestor node '("default_variant" "selector_variant"))
    (when-let ((placeable (fluent-ts-mode--placeable-for-selector node)))
      (cons (treesit-node-start placeable)
            (* 2 fluent-ts-mode-indent-offset))))
   ((when-let ((attribute (fluent-ts-mode--ancestor node '("attribute"))))
      (cons (treesit-node-start attribute) fluent-ts-mode-indent-offset)))
   ((when-let ((entry (fluent-ts-mode--ancestor node '("message" "term"))))
      (cons (treesit-node-start entry) fluent-ts-mode-indent-offset)))
   (t nil)))

(defun fluent-ts-mode--indent (node parent bol)
  "Return a Tree-sitter indentation anchor for NODE at BOL.

PARENT is NODE's parent.  The return value is an (ANCHOR . OFFSET)
pair suitable for `treesit-indent-function'."
  (let* ((context (or node parent))
         (type (and node (treesit-node-type node)))
         (character (char-after bol)))
    (cond
     ((null context) (cons bol 0))
     ((member type '("message" "term" "comment_block" "doc_commented"
                     "file_comment" "group_comment"))
      (cons (line-beginning-position) 0))
     ((eq character ?})
      (if-let ((placeable (fluent-ts-mode--placeable-for-selector context)))
          (cons (treesit-node-start placeable) 0)
        (cons bol 0)))
     ((eq character ?*)
      (if-let ((placeable (fluent-ts-mode--placeable-for-selector context)))
          (cons (treesit-node-start placeable)
                (max 0 (1- fluent-ts-mode-indent-offset)))
        (cons bol 0)))
     ((eq character ?\[)
      (if-let ((placeable (fluent-ts-mode--placeable-for-selector context)))
          (cons (treesit-node-start placeable) fluent-ts-mode-indent-offset)
        (cons bol 0)))
     ((eq character ?.)
      (if-let ((entry-start
                (fluent-ts-mode--ancestor-start context '("message" "term"))))
          (cons entry-start fluent-ts-mode-indent-offset)
        (cons bol 0)))
     ((fluent-ts-mode--value-anchor context))
     (t (cons (line-beginning-position) 0)))))

(defun fluent-ts-mode--defun-name (node)
  "Return the identifier for Fluent definition NODE."
  (when-let ((identifier
              (treesit-node-child-by-field-name node "id")))
    (treesit-node-text identifier t)))

(defun fluent-ts-mode--enable-flymake ()
  "Enable the Fluent Flymake backend in the current buffer."
  (require 'fluent-ts-flymake)
  (add-hook 'flymake-diagnostic-functions
            #'fluent-ts-flymake-backend nil t)
  (flymake-mode 1))

(defun fluent-ts-mode--enable-flycheck ()
  "Enable the Fluent Flycheck checker in the current buffer."
  (unless (require 'flycheck nil t)
    (user-error
     "Flycheck is unavailable; install Flycheck or use `fluent-ts-mode-checker' = flymake"))
  (require 'flycheck-fluent)
  (setq-local flycheck-checker 'fluent)
  (flycheck-mode 1))

(defun fluent-ts-mode--configure-checker ()
  "Configure diagnostics according to `fluent-ts-mode-checker'."
  (pcase fluent-ts-mode-checker
    ('nil nil)
    ('flymake (fluent-ts-mode--enable-flymake))
    ('flycheck (fluent-ts-mode--enable-flycheck))
    ('auto (if (featurep 'flycheck)
               (fluent-ts-mode--enable-flycheck)
             (fluent-ts-mode--enable-flymake)))
    (_ (user-error "Invalid `fluent-ts-mode-checker' value: %S"
                   fluent-ts-mode-checker))))

;;;###autoload
(defun fluent-ts-mode-install-grammar (&optional directory)
  "Install the Fluent Tree-sitter grammar into DIRECTORY.

Without a prefix argument, install into Emacs's standard Tree-sitter
directory.  With a prefix argument, prompt for DIRECTORY.  This is the only
command in fluent-ts-mode that may access the network."
  (interactive
   (list
    (when current-prefix-arg
      (read-directory-name "Install Fluent grammar in: "
                           (locate-user-emacs-file "tree-sitter")))))
  (treesit-install-language-grammar 'fluent directory))

;;;###autoload
(define-derived-mode fluent-ts-mode prog-mode "Fluent"
  "Major mode for Fluent FTL resources, powered by Tree-sitter."
  :group 'fluent-ts
  :syntax-table fluent-ts-mode--syntax-table

  (unless (treesit-ready-p 'fluent t)
    (user-error
     (concat "Fluent Tree-sitter grammar is unavailable; "
             "run M-x fluent-ts-mode-install-grammar")))

  (treesit-parser-create 'fluent)

  (setq-local comment-start "# ")
  (setq-local comment-end "")
  (setq-local comment-start-skip (rx "#" (* "#") (* (syntax whitespace))))
  (setq-local indent-tabs-mode nil)

  (setq-local treesit-indent-function #'fluent-ts-mode--indent)

  (setq-local treesit-font-lock-settings fluent-ts-mode--font-lock-settings)
  (setq-local treesit-font-lock-feature-list
              '((comment)
                (message term attribute variable)
                (selector variant function literal escape-sequence
                          bracket operator delimiter error)
                ()))

  (setq-local treesit-defun-type-regexp fluent-ts-mode--defun-regexp)
  (setq-local treesit-defun-name-function #'fluent-ts-mode--defun-name)
  (setq-local treesit-simple-imenu-settings
              '(("Message" "\\`message\\'" nil nil)
                ("Term" "\\`term\\'" nil nil)))
  (setq-local treesit-thing-settings
              `((fluent
                 (defun ,fluent-ts-mode--defun-regexp)
                 (sexp ,(regexp-opt
                         '("attribute" "default_variant" "function_call"
                           "placeable" "selector_variant" "selectors")))
                 (sentence ,(regexp-opt '("attribute" "pattern")))
                 (text ,(regexp-opt
                         '("comment_block" "doc_comment_block"
                           "file_comment" "group_comment" "pure_text"))))))

  (treesit-major-mode-setup)
  (fluent-ts-mode--configure-checker))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.ftl\\'" . fluent-ts-mode))

(provide 'fluent-ts-mode)

;;; fluent-ts-mode.el ends here
