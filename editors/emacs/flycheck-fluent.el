;;; flycheck-fluent.el --- Flycheck checker for Fluent FTL  -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Casey Link

;; Author: Casey Link <casey@outskirtslabs.com>
;; Maintainer: Casey Link <casey@outskirtslabs.com>
;; Version: 0.1.0
;; Package-Requires: ((emacs "29.1") (flycheck "35"))
;; Keywords: languages, i18n, fluent, flycheck
;; URL: https://github.com/outskirtslabs/fluent-tooling
;; SPDX-License-Identifier: MIT

;;; Commentary:

;; Optional Flycheck support for `fluent-ts-mode'.  The checker sends unsaved
;; buffer contents to `fl-lint --format json -' and converts the stable JSON
;; protocol into Flycheck errors.

;;; Code:

(require 'flycheck)
(require 'fluent-ts-diagnostics)
(require 'fluent-ts-mode)

(defun flycheck-fluent--error (diagnostic checker buffer)
  "Convert DIAGNOSTIC from CHECKER into a Flycheck error for BUFFER."
  (let* ((label (fluent-ts-diagnostics-primary-label diagnostic))
         (span (alist-get 'span label))
         (start (alist-get 'start span))
         (end (alist-get 'end span)))
    (flycheck-error-new-at
     (1+ (or (alist-get 'line start) 0))
     (1+ (or (alist-get 'column start) 0))
     (if (equal (alist-get 'severity diagnostic) "warning")
         'warning
       'error)
     (alist-get 'message diagnostic)
     :end-line (1+ (or (alist-get 'line end)
                       (alist-get 'line start)
                       0))
     :end-column (1+ (or (alist-get 'column end)
                         (alist-get 'column start)
                         0))
     :checker checker
     :id (alist-get 'code diagnostic)
     :buffer buffer
     :filename (buffer-file-name buffer))))

(defun flycheck-fluent-parse (output checker buffer)
  "Parse fl-lint JSON OUTPUT from CHECKER for BUFFER."
  (condition-case nil
      (mapcar (lambda (diagnostic)
                (flycheck-fluent--error diagnostic checker buffer))
              (fluent-ts-diagnostics-parse output))
    (fluent-ts-diagnostics-error nil)))

(defvaralias 'flycheck-fluent-executable
  'fluent-ts-mode-linter-executable)

(flycheck-define-checker fluent
  "Check Fluent FTL syntax and structure with fl-lint."
  :command ("fl-lint" "--format" "json" "-")
  :standard-input t
  :error-parser flycheck-fluent-parse
  :modes fluent-ts-mode)

(add-to-list 'flycheck-checkers 'fluent)

(provide 'flycheck-fluent)

;;; flycheck-fluent.el ends here
