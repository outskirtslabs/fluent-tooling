;;; fluent-ts-flymake-test.el --- Tests for Fluent Flymake  -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: MIT

;;; Code:

(require 'cl-lib)
(require 'ert)
(require 'flymake)

(defconst fluent-ts-flymake-test--root
  (expand-file-name "../../.." (file-name-directory load-file-name)))

(add-to-list 'load-path
             (expand-file-name "editors/emacs" fluent-ts-flymake-test--root))

(defconst fluent-ts-flymake-test--linter
  (or (getenv "FL_LINT")
      (expand-file-name "target/debug/fl-lint" fluent-ts-flymake-test--root)))

(defun fluent-ts-flymake-test--ensure-loaded ()
  "Require the Fluent Flymake adapter."
  (should (require 'fluent-ts-flymake nil t)))

(cl-defun fluent-ts-flymake-test--run
    (source &key (executable fluent-ts-flymake-test--linter) file-name)
  "Run the Fluent Flymake backend over SOURCE.

Use EXECUTABLE as the linter and FILE-NAME as the optional visited file."
  (with-temp-buffer
    (insert source)
    (setq-local fluent-ts-mode-linter-executable executable)
    (setq buffer-file-name file-name)
    (let ((called nil)
          action
          report-properties)
      (fluent-ts-flymake-backend
       (lambda (report-action &rest properties)
         (setq called t
               action report-action
               report-properties properties)))
      (let ((deadline (+ (float-time) 5)))
        (while (and (not called) (< (float-time) deadline))
          (accept-process-output nil 0.05)))
      (should called)
      (list action report-properties))))

(defun fluent-ts-flymake-test--script (contents)
  "Create an executable temporary shell script containing CONTENTS."
  (let ((path (make-temp-file "fluent-flymake" nil ".sh" contents)))
    (set-file-modes path #o700)
    path))

(ert-deftest fluent-ts-flymake-lints-unsaved-buffer-contents ()
  (fluent-ts-flymake-test--ensure-loaded)
  (let ((visited-file (make-temp-file "fluent-visited" nil ".ftl"
                                      "saved = Clean\n")))
    (unwind-protect
        (pcase-let* ((`(,diagnostics ,_)
                      (fluent-ts-flymake-test--run
                       "unsaved-message ="
                       :file-name visited-file))
                     (diagnostic (car diagnostics)))
          (should (= (length diagnostics) 1))
          (should (eq (flymake-diagnostic-type diagnostic) :error))
          (should (string-match-p "E0005" (flymake-diagnostic-text diagnostic)))
          (should (= (flymake-diagnostic-beg diagnostic)
                     (flymake-diagnostic-end diagnostic))))
      (delete-file visited-file))))

(ert-deftest fluent-ts-flymake-preserves-warning-severity-and-code ()
  (fluent-ts-flymake-test--ensure-loaded)
  (pcase-let* ((source (concat
                        "message = { PLURAL($people) ->\n"
                        "   *[other] Other\n"
                        "}\n"))
               (`(,diagnostics ,_)
                (fluent-ts-flymake-test--run source))
               (warning (seq-find
                         (lambda (diagnostic)
                           (string-match-p "W1001"
                                           (flymake-diagnostic-text diagnostic)))
                         diagnostics)))
    (should warning)
    (should (eq (flymake-diagnostic-type warning) :warning))
    (should (equal (plist-get (flymake-diagnostic-data warning) :code)
                   "W1001"))))

(ert-deftest fluent-ts-flymake-reports-a-clean-buffer ()
  (fluent-ts-flymake-test--ensure-loaded)
  (pcase-let ((`(,diagnostics ,_)
               (fluent-ts-flymake-test--run "message = Clean\n")))
    (should-not diagnostics)))

(ert-deftest fluent-ts-flymake-cancels-an-obsolete-process ()
  (fluent-ts-flymake-test--ensure-loaded)
  (let ((script
         (fluent-ts-flymake-test--script
          (concat
           "#!/bin/sh\n"
           "input=$(cat)\n"
           "case \"$input\" in\n"
           "  *slow*)\n"
           "    sleep 0.4\n"
           "    printf '%s\\n' '{\"schema_version\":1,\"diagnostics\":[]}'\n"
           "    ;;\n"
           "  *)\n"
           "    printf '%s\\n' '{\"schema_version\":1,\"diagnostics\":[]}'\n"
           "    ;;\n"
           "esac\n"))))
    (unwind-protect
        (with-temp-buffer
          (insert "slow = Value\n")
          (setq-local fluent-ts-mode-linter-executable script)
          (let ((obsolete-called nil)
                (current-called nil))
            (fluent-ts-flymake-backend
             (lambda (&rest _report) (setq obsolete-called t)))
            (let ((obsolete-process fluent-ts-flymake--process))
              (erase-buffer)
              (insert "clean = Value\n")
              (fluent-ts-flymake-backend
               (lambda (&rest _report) (setq current-called t)))
              (let ((deadline (+ (float-time) 5)))
                (while (and (not current-called) (< (float-time) deadline))
                  (accept-process-output nil 0.05)))
              (accept-process-output nil 0.5)
              (should current-called)
              (should-not obsolete-called)
              (should-not (process-live-p obsolete-process)))))
      (delete-file script))))

(ert-deftest fluent-ts-flymake-handles-malformed-json ()
  (fluent-ts-flymake-test--ensure-loaded)
  (let ((script
         (fluent-ts-flymake-test--script
          "#!/bin/sh\ncat >/dev/null\nprintf 'not json\\n'\nexit 1\n")))
    (unwind-protect
        (pcase-let ((`(,action ,properties)
                     (fluent-ts-flymake-test--run
                      "message = Value\n" :executable script)))
          (should (eq action :panic))
          (should (string-match-p
                   "invalid JSON"
                   (plist-get properties :explanation))))
      (delete-file script))))

(ert-deftest fluent-ts-flymake-handles-a-missing-executable ()
  (fluent-ts-flymake-test--ensure-loaded)
  (pcase-let ((`(,action ,properties)
               (fluent-ts-flymake-test--run
                "message = Value\n"
                :executable "/no/such/fl-lint")))
    (should (eq action :panic))
    (should (string-match-p
             "Cannot start Fluent linter"
             (plist-get properties :explanation)))))

(ert-deftest fluent-ts-mode-can-enable-flymake-explicitly ()
  (fluent-ts-flymake-test--ensure-loaded)
  (require 'treesit)
  (add-to-list 'treesit-extra-load-path
               (expand-file-name "target/emacs-tree-sitter"
                                 fluent-ts-flymake-test--root))
  (let ((fluent-ts-mode-checker 'flymake)
        (flymake-start-on-flymake-mode nil))
    (with-temp-buffer
      (insert "message = Clean\n")
      (fluent-ts-mode)
      (should flymake-mode)
      (should (memq #'fluent-ts-flymake-backend
                    flymake-diagnostic-functions)))))

(ert-deftest fluent-ts-mode-auto-falls-back-to-flymake-without-flycheck ()
  (fluent-ts-flymake-test--ensure-loaded)
  (let ((output (generate-new-buffer " *fluent auto flymake*"))
        (library-directory
         (expand-file-name "editors/emacs" fluent-ts-flymake-test--root))
        (grammar-directory
         (expand-file-name "target/emacs-tree-sitter"
                           fluent-ts-flymake-test--root)))
    (unwind-protect
        (let ((status
               (call-process
                (expand-file-name invocation-name invocation-directory)
                nil output nil
                "--batch" "-Q"
                "--directory" library-directory
                "--eval"
                (format
                 (concat
                  "(progn (require 'treesit) "
                  "(setq treesit-extra-load-path '(%S) "
                  "flymake-start-on-flymake-mode nil "
                  "fluent-ts-mode-checker 'auto) "
                  "(require 'fluent-ts-mode) "
                  "(with-temp-buffer (insert \"message = Clean\\n\") "
                  "(fluent-ts-mode) "
                  "(prin1 (list flymake-mode "
                  "(and (memq #'fluent-ts-flymake-backend "
                  "flymake-diagnostic-functions) t) "
                  "(featurep 'flycheck))))))")
                 grammar-directory))))
          (should (zerop status))
          (with-current-buffer output
            (should (equal (buffer-string) "(t t nil)"))))
      (kill-buffer output))))

(provide 'fluent-ts-flymake-test)

;;; fluent-ts-flymake-test.el ends here
