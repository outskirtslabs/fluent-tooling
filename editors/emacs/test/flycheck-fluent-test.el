;;; flycheck-fluent-test.el --- Tests for the Fluent Flycheck checker  -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: MIT

;;; Code:

(require 'ert)
(require 'flycheck)

(defconst flycheck-fluent-test--root
  (expand-file-name "../../.." (file-name-directory load-file-name)))

(defconst flycheck-fluent-test--library-directory
  (expand-file-name "editors/emacs" flycheck-fluent-test--root))

(add-to-list 'load-path flycheck-fluent-test--library-directory)

(defun flycheck-fluent-test--ensure-loaded ()
  "Require the Fluent Flycheck adapter."
  (should (require 'flycheck-fluent nil t)))

(defconst flycheck-fluent-test--json
  (concat
   "{\"schema_version\":1,\"diagnostics\":["
   "{\"path\":\"<stdin>\",\"severity\":\"error\","
   "\"code\":\"E0003\",\"message\":\"expected token `}`\","
   "\"labels\":[{\"primary\":true,\"message\":\"unexpected `?`\","
   "\"span\":{\"start\":{\"byte\":30,\"line\":0,\"column\":26},"
   "\"end\":{\"byte\":31,\"line\":0,\"column\":27}}}],"
   "\"notes\":[],\"help\":[\"close the placeable\"]},"
   "{\"path\":\"<stdin>\",\"severity\":\"warning\","
   "\"code\":\"W1001\",\"message\":\"explicit PLURAL is unnecessary\","
   "\"labels\":[{\"primary\":true,\"message\":\"remove PLURAL\","
   "\"span\":{\"start\":{\"byte\":0,\"line\":1,\"column\":0},"
   "\"end\":{\"byte\":6,\"line\":1,\"column\":6}}}],"
   "\"notes\":[],\"help\":[]}]}"))

(ert-deftest flycheck-fluent-converts-json-severity-code-and-unicode-positions ()
  (flycheck-fluent-test--ensure-loaded)
  (with-temp-buffer
    (insert "message = Lubię 😂 { $name ? }\nPLURAL\n")
    (let* ((errors (flycheck-fluent-parse flycheck-fluent-test--json
                                           'fluent (current-buffer)))
           (error (car errors))
           (warning (cadr errors)))
      (should (= (length errors) 2))
      (should (eq (flycheck-error-level error) 'error))
      (should (equal (flycheck-error-id error) "E0003"))
      (should (= (flycheck-error-line error) 1))
      (should (= (flycheck-error-column error) 27))
      (should (= (flycheck-error-end-line error) 1))
      (should (= (flycheck-error-end-column error) 28))
      (should (eq (flycheck-error-buffer error) (current-buffer)))
      (should (equal (flycheck-error-message error) "expected token `}`"))
      (should (eq (flycheck-error-level warning) 'warning))
      (should (equal (flycheck-error-id warning) "W1001"))
      (should (= (flycheck-error-line warning) 2))
      (should (= (flycheck-error-column warning) 1)))))

(ert-deftest flycheck-fluent-ignores-malformed-or-unsupported-json ()
  (flycheck-fluent-test--ensure-loaded)
  (with-temp-buffer
    (should-not (flycheck-fluent-parse "not json" 'fluent (current-buffer)))
    (should-not
     (flycheck-fluent-parse
      "{\"schema_version\":2,\"diagnostics\":[]}" 'fluent
      (current-buffer)))))

(ert-deftest flycheck-fluent-registers-an-stdin-command-checker ()
  (flycheck-fluent-test--ensure-loaded)
  (should (flycheck-valid-checker-p 'fluent))
  (should (memq 'fluent flycheck-checkers))
  (should (equal (flycheck-checker-get 'fluent 'modes)
                 '(fluent-ts-mode)))
  (should (flycheck-checker-get 'fluent 'standard-input))
  (should (equal (flycheck-checker-get 'fluent 'command)
                 '("fl-lint" "--format" "json" "-")))
  (let ((fluent-ts-mode-linter-executable "/tmp/custom-fl-lint"))
    (should (equal flycheck-fluent-executable
                   "/tmp/custom-fl-lint"))))

(ert-deftest fluent-ts-mode-auto-prefers-loaded-flycheck ()
  (flycheck-fluent-test--ensure-loaded)
  (require 'treesit)
  (add-to-list 'treesit-extra-load-path
               (expand-file-name "target/emacs-tree-sitter"
                                 flycheck-fluent-test--root))
  (let ((fluent-ts-mode-checker 'auto)
        (flycheck-check-syntax-automatically nil))
    (with-temp-buffer
      (insert "message = Clean\n")
      (fluent-ts-mode)
      (should flycheck-mode)
      (should (eq flycheck-checker 'fluent))
      (should-not (bound-and-true-p flymake-mode)))))

(ert-deftest fluent-ts-mode-can-disable-checkers ()
  (flycheck-fluent-test--ensure-loaded)
  (require 'treesit)
  (add-to-list 'treesit-extra-load-path
               (expand-file-name "target/emacs-tree-sitter"
                                 flycheck-fluent-test--root))
  (let ((fluent-ts-mode-checker nil)
        (flycheck-check-syntax-automatically nil))
    (with-temp-buffer
      (insert "message = Clean\n")
      (fluent-ts-mode)
      (should-not flycheck-mode)
      (should-not (bound-and-true-p flymake-mode)))))

(ert-deftest fluent-ts-mode-does-not-require-flycheck ()
  (flycheck-fluent-test--ensure-loaded)
  (let ((output (generate-new-buffer " *fluent optional flycheck*")))
    (unwind-protect
        (let ((status
               (call-process
                (expand-file-name invocation-name invocation-directory)
                nil output nil
                "--batch" "-Q"
                "--directory" flycheck-fluent-test--library-directory
                "--eval"
                "(progn (require 'fluent-ts-mode) (prin1 (featurep 'flycheck)))")))
          (should (zerop status))
          (with-current-buffer output
            (should (equal (buffer-string) "nil"))))
      (kill-buffer output))))

(provide 'flycheck-fluent-test)

;;; flycheck-fluent-test.el ends here
