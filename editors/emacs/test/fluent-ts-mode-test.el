;;; fluent-ts-mode-test.el --- Tests for fluent-ts-mode  -*- lexical-binding: t; -*-

;; SPDX-License-Identifier: MIT

;;; Code:

(require 'ert)
(require 'imenu)
(require 'treesit)

(defconst fluent-ts-mode-test--root
  (expand-file-name "../../.." (file-name-directory load-file-name)))

(defconst fluent-ts-mode-test--library-directory
  (expand-file-name "editors/emacs" fluent-ts-mode-test--root))

(defconst fluent-ts-mode-test--grammar-directory
  (or (getenv "FLUENT_TS_GRAMMAR_DIR")
      (expand-file-name "target/emacs-tree-sitter" fluent-ts-mode-test--root)))

(add-to-list 'load-path fluent-ts-mode-test--library-directory)
(add-to-list 'treesit-extra-load-path fluent-ts-mode-test--grammar-directory)

(defun fluent-ts-mode-test--ensure-loaded ()
  "Require `fluent-ts-mode' and verify the test grammar is ready."
  (should (require 'fluent-ts-mode nil t))
  (should (treesit-ready-p 'fluent t)))

(defmacro fluent-ts-mode-test--with-buffer (source &rest body)
  "Create a Fluent buffer containing SOURCE, then evaluate BODY."
  (declare (indent 1) (debug t))
  `(with-temp-buffer
     (insert ,source)
     (fluent-ts-mode)
     ,@body))

(defun fluent-ts-mode-test--face-at (text &optional occurrence)
  "Return the face at TEXT's OCCURRENCE in the current buffer."
  (goto-char (point-min))
  (dotimes (_ (or occurrence 1))
    (should (search-forward text nil t)))
  (font-lock-ensure)
  (get-text-property (- (point) (length text)) 'face))

(defun fluent-ts-mode-test--has-face-p (face value)
  "Return non-nil when VALUE contains FACE."
  (if (listp value)
      (memq face value)
    (eq face value)))

(defun fluent-ts-mode-test--index-names (category index)
  "Return entry names for CATEGORY in Imenu INDEX."
  (mapcar #'car (cdr (assoc category index))))

(ert-deftest fluent-ts-mode-activates-with-a-local-grammar ()
  (fluent-ts-mode-test--ensure-loaded)
  (fluent-ts-mode-test--with-buffer "hello = Hello\n"
    (should (eq major-mode 'fluent-ts-mode))
    (should (eq (treesit-parser-language (car (treesit-parser-list)))
                'fluent))
    (should (equal comment-start "# "))
    (should (equal comment-end ""))
    (should-not indent-tabs-mode)
    (should (= fluent-ts-mode-indent-offset 4))
    (should (equal fluent-ts-mode-linter-executable "ftl-lint"))))

(ert-deftest fluent-ts-mode-reports-a-clear-missing-grammar-error ()
  (fluent-ts-mode-test--ensure-loaded)
  (let ((output (generate-new-buffer " *fluent missing grammar*"))
        (temporary-user-directory (make-temp-file "fluent-emacs-home" t))
        (mode-file (expand-file-name "fluent-ts-mode.el"
                                     fluent-ts-mode-test--library-directory)))
    (unwind-protect
        (let ((status
               (call-process
                (expand-file-name invocation-name invocation-directory)
                nil output nil
                "--batch" "-Q"
                "--eval"
                (format "(setq user-emacs-directory %S treesit-extra-load-path nil)"
                        temporary-user-directory)
                "--load" mode-file
                "--eval" "(with-temp-buffer (fluent-ts-mode))")))
          (with-current-buffer output
            (let ((message (buffer-string)))
              (should-not (zerop status))
              (should (string-match-p
                       "Fluent Tree-sitter grammar is unavailable" message))
              (should (string-match-p
                       "fluent-ts-mode-install-grammar" message))
              (should-not (string-match-p "Cloning repository" message)))))
      (kill-buffer output)
      (delete-directory temporary-user-directory t))))

(ert-deftest fluent-ts-mode-font-locks-fluent-constructs-and-errors ()
  (fluent-ts-mode-test--ensure-loaded)
  (fluent-ts-mode-test--with-buffer
      (concat
       "### Resource comment\n"
       "# Message comment\n"
       "hello = Hello, { $name } { NUMBER($count) }\n"
       "    .title = { $count ->\n"
       "       *[one] { \"One\" }\n"
       "        [other] { 2 }\n"
       "    }\n"
       "-brand = Fluent\n"
       "broken = { $value ? }\n")
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-comment-face
             (fluent-ts-mode-test--face-at "### Resource comment")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-variable-name-face
             (fluent-ts-mode-test--face-at "hello")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-constant-face
             (fluent-ts-mode-test--face-at "-brand")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-property-name-face
             (fluent-ts-mode-test--face-at "title")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-variable-use-face
             (fluent-ts-mode-test--face-at "$name")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-keyword-face
             (fluent-ts-mode-test--face-at "->")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-type-face
             (fluent-ts-mode-test--face-at "one")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-function-call-face
             (fluent-ts-mode-test--face-at "NUMBER")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-string-face
             (fluent-ts-mode-test--face-at "\"One\"")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-number-face
             (fluent-ts-mode-test--face-at "2")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-bracket-face
             (fluent-ts-mode-test--face-at "{")))
    (should (fluent-ts-mode-test--has-face-p
             'font-lock-warning-face
             (fluent-ts-mode-test--face-at "?")))))

(ert-deftest fluent-ts-mode-indents-patterns-attributes-and-nested-selectors ()
  (fluent-ts-mode-test--ensure-loaded)
  (fluent-ts-mode-test--with-buffer
      (concat
       "message =\n"
       " First line\n"
       " .attribute =\n"
       "  Attribute continuation\n"
       "  { $outer ->\n"
       "    *[one]\n"
       "      { $inner ->\n"
       "        *[other] Other\n"
       "      }\n"
       "  }\n")
    (indent-region (point-min) (point-max))
    (should
     (equal
      (buffer-string)
      (concat
       "message =\n"
       "    First line\n"
       "    .attribute =\n"
       "        Attribute continuation\n"
       "        { $outer ->\n"
       "           *[one]\n"
       "                { $inner ->\n"
       "                   *[other] Other\n"
       "                }\n"
       "        }\n")))))

(ert-deftest fluent-ts-mode-builds-message-and-term-imenu-categories ()
  (fluent-ts-mode-test--ensure-loaded)
  (fluent-ts-mode-test--with-buffer
      (concat
       "first-message = One\n\n"
       "-brand-name = Fluent\n\n"
       "attribute-only =\n"
       "    .title = Title\n")
    (let ((index (funcall imenu-create-index-function)))
      (should (equal (fluent-ts-mode-test--index-names "Message" index)
                     '("first-message" "attribute-only")))
      (should (equal (fluent-ts-mode-test--index-names "Term" index)
                     '("-brand-name"))))))

(ert-deftest fluent-ts-mode-navigates-message-and-term-defuns ()
  (fluent-ts-mode-test--ensure-loaded)
  (fluent-ts-mode-test--with-buffer
      (concat
       "first = One\n\n"
       "-second = Two\n\n"
       "third = Three\n")
    (goto-char (point-min))
    (search-forward "Two")
    (beginning-of-defun)
    (should (looking-at-p "-second"))
    (end-of-defun)
    (should (looking-at-p "third"))))

(ert-deftest fluent-ts-mode-configures-tree-sitter-syntax-navigation ()
  (fluent-ts-mode-test--ensure-loaded)
  (fluent-ts-mode-test--with-buffer "message = Before { $name } after\n"
    (should (eq forward-sexp-function #'treesit-forward-sexp))
    (goto-char (point-min))
    (search-forward "{")
    (backward-char)
    (forward-sexp)
    (should (looking-at-p " after"))))

(ert-deftest fluent-ts-mode-registers-ftl-files-and-public-recipe ()
  (fluent-ts-mode-test--ensure-loaded)
  (should (eq (assoc-default "messages.ftl" auto-mode-alist #'string-match)
              'fluent-ts-mode))
  (should
   (equal
    (cadr (assq 'fluent treesit-language-source-alist))
    "https://github.com/outskirtslabs/fluent-tooling"))
  (should (commandp #'fluent-ts-mode-install-grammar)))

(provide 'fluent-ts-mode-test)

;;; fluent-ts-mode-test.el ends here
