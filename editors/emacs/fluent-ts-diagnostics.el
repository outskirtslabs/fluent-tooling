;;; fluent-ts-diagnostics.el --- Fluent JSON diagnostics helpers  -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Casey Link

;; Author: Casey Link <casey@outskirtslabs.com>
;; Maintainer: Casey Link <casey@outskirtslabs.com>
;; Version: 0.1.0
;; Package-Requires: ((emacs "29.1"))
;; Keywords: languages, i18n, fluent
;; URL: https://github.com/outskirtslabs/fluent-tooling
;; SPDX-License-Identifier: MIT

;;; Commentary:

;; Internal helpers shared by the Flymake and Flycheck adapters.  The parser
;; accepts version 1 of fl-lint's stable JSON editor protocol.

;;; Code:

(require 'json)
(require 'seq)

(define-error 'fluent-ts-diagnostics-error
              "Invalid fl-lint JSON diagnostics")

(defun fluent-ts-diagnostics-parse (output)
  "Parse version 1 fl-lint JSON OUTPUT and return its diagnostics."
  (let ((document
         (condition-case error-data
             (json-parse-string output
                                :object-type 'alist
                                :array-type 'list
                                :null-object nil
                                :false-object nil)
           (error
            (signal 'fluent-ts-diagnostics-error
                    (list (format "invalid JSON: %s"
                                  (error-message-string error-data))))))))
    (unless (eql (alist-get 'schema_version document) 1)
      (signal 'fluent-ts-diagnostics-error
              '("unsupported fl-lint JSON schema version")))
    (let ((diagnostics (alist-get 'diagnostics document 'missing)))
      (unless (listp diagnostics)
        (signal 'fluent-ts-diagnostics-error
                '("diagnostics must be an array")))
      diagnostics)))

(defun fluent-ts-diagnostics-primary-label (diagnostic)
  "Return DIAGNOSTIC's primary label, or its first label."
  (let ((labels (alist-get 'labels diagnostic)))
    (or (seq-find (lambda (label) (alist-get 'primary label)) labels)
        (car labels))))

(provide 'fluent-ts-diagnostics)

;;; fluent-ts-diagnostics.el ends here
