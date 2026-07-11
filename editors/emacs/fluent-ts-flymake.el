;;; fluent-ts-flymake.el --- Flymake backend for Fluent FTL  -*- lexical-binding: t; -*-

;; Copyright (C) 2026 Casey Link

;; Author: Casey Link <casey@outskirtslabs.com>
;; Maintainer: Casey Link <casey@outskirtslabs.com>
;; Version: 0.1.0
;; Package-Requires: ((emacs "29.1"))
;; Keywords: languages, i18n, fluent, flymake
;; URL: https://github.com/outskirtslabs/fluent-tooling
;; SPDX-License-Identifier: MIT

;;; Commentary:

;; This asynchronous Flymake backend sends the current buffer to
;; `ftl-lint --format json -'.  It checks unsaved contents and cancels obsolete
;; processes when Flymake requests a newer check.

;;; Code:

(require 'flymake)
(require 'fluent-ts-diagnostics)
(require 'fluent-ts-mode)
(require 'subr-x)

(defvar-local fluent-ts-flymake--process nil
  "The active ftl-lint process for the current buffer.")

(defun fluent-ts-flymake--cancel-process ()
  "Cancel the current buffer's obsolete ftl-lint process."
  (when (processp fluent-ts-flymake--process)
    (process-put fluent-ts-flymake--process 'fluent-ts-obsolete t)
    (when (process-live-p fluent-ts-flymake--process)
      (delete-process fluent-ts-flymake--process)))
  (setq fluent-ts-flymake--process nil))

(defun fluent-ts-flymake--position (position)
  "Convert JSON POSITION to a position in the current buffer."
  (let ((line (or (alist-get 'line position) 0))
        (column (or (alist-get 'column position) 0)))
    (save-restriction
      (widen)
      (save-excursion
        (goto-char (point-min))
        (forward-line line)
        (forward-char (min column (- (line-end-position) (point))))
        (point)))))

(defun fluent-ts-flymake--diagnostic (diagnostic)
  "Convert a ftl-lint DIAGNOSTIC into a Flymake diagnostic."
  (let* ((label (fluent-ts-diagnostics-primary-label diagnostic))
         (span (alist-get 'span label))
         (start (and span (alist-get 'start span)))
         (end (and span (alist-get 'end span)))
         (beg (if start (fluent-ts-flymake--position start) (point-min)))
         (finish (if end (fluent-ts-flymake--position end) beg))
         (severity (if (equal (alist-get 'severity diagnostic) "warning")
                       :warning
                     :error))
         (code (alist-get 'code diagnostic))
         (message (alist-get 'message diagnostic)))
    (flymake-make-diagnostic
     (current-buffer) beg finish severity
     (format "%s: %s" code message)
     (list :code code
           :path (alist-get 'path diagnostic)
           :notes (alist-get 'notes diagnostic)
           :help (alist-get 'help diagnostic)))))

(defun fluent-ts-flymake--buffer-string (buffer)
  "Return BUFFER's contents, or an empty string if BUFFER is dead."
  (if (buffer-live-p buffer)
      (with-current-buffer buffer (buffer-string))
    ""))

(defun fluent-ts-flymake--cleanup-process (process)
  "Release buffers and state associated with PROCESS."
  (let ((source (process-get process 'fluent-ts-source-buffer)))
    (when (buffer-live-p source)
      (with-current-buffer source
        (when (eq fluent-ts-flymake--process process)
          (setq fluent-ts-flymake--process nil)))))
  (dolist (buffer (list (process-buffer process)
                        (process-get process 'fluent-ts-stderr-buffer)))
    (when (buffer-live-p buffer)
      (kill-buffer buffer))))

(defun fluent-ts-flymake--sentinel (process _event)
  "Report diagnostics when PROCESS exits."
  (unless (process-live-p process)
    (unwind-protect
        (unless (process-get process 'fluent-ts-obsolete)
          (let ((source (process-get process 'fluent-ts-source-buffer))
                (report-fn (process-get process 'fluent-ts-report-fn))
                (status (process-exit-status process)))
            (when (buffer-live-p source)
              (with-current-buffer source
                (if (memq status '(0 1))
                    (condition-case error-data
                        (funcall
                         report-fn
                         (mapcar #'fluent-ts-flymake--diagnostic
                                 (fluent-ts-diagnostics-parse
                                  (fluent-ts-flymake--buffer-string
                                   (process-buffer process)))))
                      (fluent-ts-diagnostics-error
                       (funcall report-fn :panic
                                :explanation
                                (error-message-string error-data))))
                  (let ((stderr
                         (string-trim
                          (fluent-ts-flymake--buffer-string
                           (process-get process 'fluent-ts-stderr-buffer)))))
                    (funcall
                     report-fn :panic
                     :explanation
                     (format "Fluent linter failed with exit status %d%s"
                             status
                             (if (string-empty-p stderr)
                                 ""
                               (format ": %s" stderr))))))))))
      (fluent-ts-flymake--cleanup-process process))))

;;;###autoload
(defun fluent-ts-flymake-backend (report-fn &rest _arguments)
  "Run ftl-lint for Flymake and call REPORT-FN with its diagnostics."
  (fluent-ts-flymake--cancel-process)
  (let ((source (current-buffer))
        (stdout (generate-new-buffer " *fluent-ts-flymake stdout*"))
        (stderr (generate-new-buffer " *fluent-ts-flymake stderr*")))
    (condition-case error-data
        (let ((process
               (make-process
                :name "fluent-ts-flymake"
                :buffer stdout
                :stderr stderr
                :command (list fluent-ts-mode-linter-executable
                               "--format" "json" "-")
                :connection-type 'pipe
                :noquery t
                :sentinel #'fluent-ts-flymake--sentinel)))
          (setq fluent-ts-flymake--process process)
          (process-put process 'fluent-ts-source-buffer source)
          (process-put process 'fluent-ts-stderr-buffer stderr)
          (process-put process 'fluent-ts-report-fn report-fn)
          (save-restriction
            (widen)
            (process-send-region process (point-min) (point-max)))
          (process-send-eof process))
      (error
       (when (buffer-live-p stdout) (kill-buffer stdout))
       (when (buffer-live-p stderr) (kill-buffer stderr))
       (funcall report-fn :panic
                :explanation
                (format "Cannot start Fluent linter `%s`: %s"
                        fluent-ts-mode-linter-executable
                        (error-message-string error-data)))))))

(provide 'fluent-ts-flymake)

;;; fluent-ts-flymake.el ends here
