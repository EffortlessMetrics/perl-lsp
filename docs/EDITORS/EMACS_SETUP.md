# Emacs Setup Guide for perl-lsp

This comprehensive guide helps you set up and configure the Perl Language Server in Emacs.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Basic Setup](#basic-setup)
- [Configuration](#configuration)
- [Features](#features)
- [Keybindings](#keybindings)
- [Packages](#packages)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

---

## Prerequisites

### Required

- **Emacs** version 27 or later (Emacs 29+ recommended for eglot)
- **perl-lsp** server installed (see [Installation](#installation))

### Optional but Recommended

- **lsp-mode** or **eglot** (LSP client)
- **lsp-ui** (enhanced UI for lsp-mode)
- **company-mode** (completion framework)
- **yasnippet** (snippet support)
- **flycheck** (syntax checking)
- **Perl** 5.10 or later (for syntax validation)
- **perltidy** (for code formatting)
- **perlcritic** (for linting)

---

## Installation

### Install the Server

Choose one of the following methods:

#### Option 1: Install from crates.io (Recommended)

```bash
cargo install perl-lsp
```

#### Option 2: Download Pre-built Binary

Download from [GitHub Releases](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases):

```bash
# Linux (x86_64)
curl -LO https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases/latest/download/perl-lsp-linux-x86_64.tar.gz
tar xzf perl-lsp-linux-x86_64.tar.gz
sudo mv perl-lsp /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases/latest/download/perl-lsp-darwin-aarch64.tar.gz
tar xzf perl-lsp-darwin-aarch64.tar.gz
sudo mv perl-lsp /usr/local/bin/
```

#### Option 3: Build from Source

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl-rs.git
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp
```

### Verify Installation

```bash
# Check version
perl-lsp --version

# Quick health check
perl-lsp --health
# Should output: ok 0.9.0
```

---

## Basic Setup

### Option 1: Using lsp-mode (Recommended for Emacs 27-28)

Add to your Emacs configuration (`~/.emacs.d/init.el` or `~/.config/emacs/init.el`):

```elisp
(use-package lsp-mode
  :ensure t
  :hook ((cperl-mode . lsp-deferred)
         (perl-mode . lsp-deferred))
  :commands lsp
  :init
  (setq lsp-keymap-prefix "C-c l")
  :config
  ;; Register perl-lsp
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perl-lsp" "--stdio"))
    :major-modes '(cperl-mode perl-mode)
    :priority -1
    :server-id 'perl-lsp
    :initialization-options
    '((perl
       (workspace
        (includePaths . ["lib" "." "local/lib/perl5"])
        (useSystemInc . :json-false)
        (resolutionTimeout . 50))
       (inlayHints
        (enabled . t)
        (parameterHints . t)
        (typeHints . t))
       (limits
        (workspaceSymbolCap . 200)
        (referencesCap . 500)
        (completionCap . 100)))))))

;; Optional: Enable lsp-ui for enhanced UI
(use-package lsp-ui
  :ensure t
  :hook (lsp-mode . lsp-ui-mode)
  :config
  (setq lsp-ui-doc-enable t
        lsp-ui-doc-show-with-cursor t
        lsp-ui-sideline-enable t))
```

### Option 2: Using eglot (Recommended for Emacs 29+)

Add to your Emacs configuration:

```elisp
(use-package eglot
  :ensure t
  :hook ((cperl-mode . eglot-ensure)
         (perl-mode . eglot-ensure))
  :config
  (add-to-list 'eglot-server-programs
               '((cperl-mode perl-mode) . ("perl-lsp" "--stdio")))

  ;; Optional: Configure initialization options
  (setq-default eglot-workspace-configuration
    '((perl
       (workspace
        (includePaths . ["lib" "." "local/lib/perl5"])
        (useSystemInc . :json-false))
       (limits
        (workspaceSymbolCap . 200)
        (referencesCap . 500))))))
```

### Verify Setup

1. Restart Emacs
2. Open a `.pl` or `.pm` file
3. Start LSP:
   - **lsp-mode**: `M-x lsp`
   - **eglot**: Starts automatically
4. Check if LSP is attached:
   - **lsp-mode**: `M-x lsp-describe-session`
   - **eglot**: `M-x eglot`

---

## Configuration

### Full lsp-mode Configuration

```elisp
(use-package lsp-mode
  :ensure t
  :hook ((cperl-mode . lsp-deferred)
         (perl-mode . lsp-deferred))
  :commands lsp
  :init
  (setq lsp-keymap-prefix "C-c l"
        lsp-auto-guess-root t
        lsp-prefer-flymake nil
        lsp-enable-file-watchers t
        lsp-enable-folding nil
        lsp-enable-snippet nil
        lsp-enable-symbol-highlighting t
        lsp-enable-text-document-color nil
        lsp-enable-on-type-formatting nil
        lsp-modeline-code-actions-enable t
        lsp-modeline-diagnostics-enable t
        lsp-modeline-workspace-status-enable t
        lsp-semantic-tokens-enable t
        lsp-signature-auto-activate t
        lsp-signature-render-documentation t
        lsp-completion-provider :capf
        lsp-completion-show-kind t
        lsp-completion-show-detail t
        lsp-completion-show-label-description t
        lsp-completion-enable-additional-text-edit t
        lsp-idle-delay 0.5)
  :config
  ;; Register perl-lsp
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perl-lsp" "--stdio"))
    :major-modes '(cperl-mode perl-mode)
    :priority -1
    :server-id 'perl-lsp
    :initialization-options
    '((perl
       (workspace
        (includePaths . ["lib" "." "local/lib/perl5"])
        (useSystemInc . :json-false)
        (resolutionTimeout . 50))
       (inlayHints
        (enabled . t)
        (parameterHints . t)
        (typeHints . t)
        (chainedHints . :json-false)
        (maxLength . 30))
       (testRunner
        (enabled . t)
        (command . "perl")
        (args . [])
        (timeout . 60000))
       (limits
        (workspaceSymbolCap . 200)
        (referencesCap . 500)
        (completionCap . 100)
        (astCacheMaxEntries . 100)
        (maxIndexedFiles . 10000)
        (maxTotalSymbols . 500000)
        (workspaceScanDeadlineMs . 30000)
        (referenceSearchDeadlineMs . 2000)))))))

  ;; Custom keybindings
  (define-key lsp-mode-map (kbd "C-c l") lsp-command-map))

;; lsp-ui for enhanced UI
(use-package lsp-ui
  :ensure t
  :hook (lsp-mode . lsp-ui-mode)
  :config
  (setq lsp-ui-doc-enable t
        lsp-ui-doc-show-with-cursor t
        lsp-ui-doc-show-with-mouse t
        lsp-ui-doc-include-signature t
        lsp-ui-doc-position 'top
        lsp-ui-doc-border t
        lsp-ui-doc-max-width 80
        lsp-ui-doc-max-height 20
        lsp-ui-sideline-enable t
        lsp-ui-sideline-show-code-actions t
        lsp-ui-sideline-show-diagnostics t
        lsp-ui-sideline-show-hover t
        lsp-ui-sideline-delay 0.1
        lsp-ui-peek-enable t
        lsp-ui-peek-list-width 50
        lsp-ui-peek-peek-height 25))

;; Company for completion
(use-package company
  :ensure t
  :hook (lsp-mode . company-mode)
  :config
  (setq company-minimum-prefix-length 1
        company-tooltip-align-annotations t
        company-tooltip-limit 20
        company-idle-delay 0.2
        company-show-numbers t
        company-selection-wrap-around t))

;; Company-lsp for LSP completion
(use-package company-lsp
  :ensure t
  :after company
  :config
  (push 'company-lsp company-backends)
  (setq company-lsp-cache-candidates 'auto))
```

### Full eglot Configuration

```elisp
(use-package eglot
  :ensure t
  :hook ((cperl-mode . eglot-ensure)
         (perl-mode . eglot-ensure))
  :init
  (setq eglot-autoshutdown t
        eglot-events-buffer-size 0
        eglot-extend-to-xref nil
        eglot-ignored-server-capabilities '(:documentHighlightProvider)
        eglot-connect-timeout 30
        eglot-sync-connect 1
        eglot-send-changes-idle-time 0.5)
  :config
  (add-to-list 'eglot-server-programs
               '((cperl-mode perl-mode) . ("perl-lsp" "--stdio")))

  ;; Workspace configuration
  (setq-default eglot-workspace-configuration
    '((perl
       (workspace
        (includePaths . ["lib" "." "local/lib/perl5"])
        (useSystemInc . :json-false)
        (resolutionTimeout . 50))
       (inlayHints
        (enabled . t)
        (parameterHints . t)
        (typeHints . t)
        (maxLength . 30))
       (testRunner
        (enabled . t)
        (command . "perl")
        (args . [])
        (timeout . 60000))
       (limits
        (workspaceSymbolCap . 200)
        (referencesCap . 500)
        (completionCap . 100)
        (astCacheMaxEntries . 100)
        (maxIndexedFiles . 10000)
        (maxTotalSymbols . 500000)
        (workspaceScanDeadlineMs . 30000)
        (referenceSearchDeadlineMs . 2000))))))

;; Company for completion
(use-package company
  :ensure t
  :hook (eglot-managed-mode . company-mode)
  :config
  (setq company-minimum-prefix-length 1
        company-tooltip-align-annotations t
        company-tooltip-limit 20
        company-idle-delay 0.2
        company-show-numbers t
        company-selection-wrap-around t))
```

### Project-Specific Configuration

Create `.dir-locals.el` in your project root:

```elisp
((perl-mode . ((eval . (lsp)))
             (perl-lsp-include-paths . ("lib" "local/lib/perl5" "vendor/lib"))
             (perl-lsp-use-system-inc . nil)))
 (cperl-mode . ((eval . (lsp)))
              (perl-lsp-include-paths . ("lib" "local/lib/perl5" "vendor/lib"))
              (perl-lsp-use-system-inc . nil))))
```

---

## Features

### Syntax Diagnostics

Real-time syntax error detection and reporting:

```perl
# Errors are highlighted as you type
my $x = 1
# Missing semicolon - error shown immediately
```

View diagnostics:
- **lsp-mode**: `M-x lsp-ui-doc-glance`
- **eglot**: `M-x flycheck-list-errors`

### Go to Definition

Navigate to symbol definitions:

```elisp
;; lsp-mode
M-x lsp-find-definition

;; eglot
M-x xref-find-definitions
```

Or use keybinding: `M-.`

```perl
use MyModule;

MyModule::some_function();
# ^ M-. here jumps to the definition
```

### Find References

Find all usages of a symbol:

```elisp
;; lsp-mode
M-x lsp-find-references

;; eglot
M-x xref-find-references
```

Or use keybinding: `M-?`

```perl
sub my_function {
    return 42;
}

# ^ Find references here shows all calls to my_function
```

### Hover Information

View documentation and type information:

```elisp
;; lsp-mode
M-x lsp-describe-thing-at-point

;; eglot
M-x eldoc
```

Or hover with mouse.

### Code Completion

Intelligent code completion:

```elisp
;; lsp-mode with company
M-x company-complete

;; eglot with company
M-x company-complete
```

Or type to trigger automatic completion.

```perl
use MyModule;

MyModule::  # Type here for completion
```

### Document Symbols

Navigate symbols in the current file:

```elisp
;; lsp-mode
M-x lsp-treemacs-symbols

;; eglot
M-x imenu
```

### Workspace Symbols

Search symbols across the entire workspace:

```elisp
;; lsp-mode
M-x lsp-workspace-symbol

;; eglot
M-x xref-find-apropos
```

### Rename Symbol

Rename symbols across the workspace:

```elisp
;; lsp-mode
M-x lsp-rename

;; eglot
M-x eglot-rename
```

### Formatting

Format Perl code using perltidy:

```elisp
;; lsp-mode
M-x lsp-format-buffer

;; eglot
M-x eglot-format-buffer
```

### Code Actions

Quick fixes and refactorings:

```elisp
;; lsp-mode
M-x lsp-execute-code-action

;; eglot
M-x eglot-code-actions
```

Available actions:
- Extract variable
- Extract subroutine
- Optimize imports
- Add missing pragmas

### Inlay Hints

Inline type and parameter hints:

```perl
sub my_function($name, $count) {
    return "Hello, $name x$count";
}

my_function("World", 5);
# ^ Shows: my_function(/* name: */ "World", /* count: */ 5)
```

Enable inlay hints:

```elisp
;; lsp-mode
M-x lsp-inlay-hints-mode

;; eglot (requires Emacs 29+)
M-x eglot-inlay-hints-mode
```

---

## Keybindings

### Default lsp-mode Keybindings

| Action | Keybinding |
|--------|------------|
| LSP Command Prefix | `C-c l` |
| Go to Definition | `C-c l d` |
| Find References | `C-c l r` |
| Rename Symbol | `C-c l r n` |
| Format Buffer | `C-c l =` |
| Code Actions | `C-c l a` |
| Hover | `C-c l h` |
| Signature Help | `C-c l s` |
| Document Symbols | `C-c l s d` |
| Workspace Symbols | `C-c l s w` |

### Default eglot Keybindings

| Action | Keybinding |
|--------|------------|
| Go to Definition | `M-.` |
| Find References | `M-?` |
| Rename Symbol | `M-x eglot-rename` |
| Format Buffer | `C-M-\` |
| Code Actions | `M-x eglot-code-actions` |
| Hover | `M-x eldoc` |
| Document Symbols | `M-x imenu` |

### Custom Keybindings

Add to your configuration:

```elisp
;; Common LSP keybindings for perl-mode
(with-eval-after-load 'perl-mode
  (define-key perl-mode-map (kbd "M-.") 'xref-find-definitions)
  (define-key perl-mode-map (kbd "M-?") 'xref-find-references)
  (define-key perl-mode-map (kbd "C-c r") 'lsp-rename)
  (define-key perl-mode-map (kbd "C-c a") 'lsp-execute-code-action)
  (define-key perl-mode-map (kbd "C-c f") 'lsp-format-buffer)
  (define-key perl-mode-map (kbd "C-c i") 'lsp-ui-imenu))

;; cperl-mode keybindings
(with-eval-after-load 'cperl-mode
  (define-key cperl-mode-map (kbd "M-.") 'xref-find-definitions)
  (define-key cperl-mode-map (kbd "M-?") 'xref-find-references)
  (define-key cperl-mode-map (kbd "C-c r") 'lsp-rename)
  (define-key cperl-mode-map (kbd "C-c a") 'lsp-execute-code-action)
  (define-key cperl-mode-map (kbd "C-c f") 'lsp-format-buffer)
  (define-key cperl-mode-map (kbd "C-c i") 'lsp-ui-imenu))
```

---

## Packages

### lsp-ui

Enhanced UI for lsp-mode:

```elisp
(use-package lsp-ui
  :ensure t
  :hook (lsp-mode . lsp-ui-mode)
  :config
  (setq lsp-ui-doc-enable t
        lsp-ui-doc-show-with-cursor t
        lsp-ui-sideline-enable t))
```

Features:
- Hover documentation
- Sideline diagnostics
- Peek references
- Symbol highlighting

### company-mode

Completion framework:

```elisp
(use-package company
  :ensure t
  :hook (lsp-mode . company-mode)
  :config
  (setq company-minimum-prefix-length 1
        company-tooltip-align-annotations t))
```

### company-lsp

LSP completion backend:

```elisp
(use-package company-lsp
  :ensure t
  :after company
  :config
  (push 'company-lsp company-backends))
```

### yasnippet

Snippet support:

```elisp
(use-package yasnippet
  :ensure t
  :config
  (yas-global-mode 1))

(use-package lsp-mode
  :config
  (setq lsp-enable-snippet t))
```

### flycheck

Syntax checking:

```elisp
(use-package flycheck
  :ensure t
  :hook (lsp-mode . flycheck-mode))
```

### treemacs

Tree view for workspace symbols:

```elisp
(use-package lsp-treemacs
  :ensure t
  :after lsp-mode
  :config
  (lsp-treemacs-sync-mode 1))
```

---

## Troubleshooting

### Server Not Starting

**Symptoms**: No diagnostics, no completion, error in `*lsp-log*` buffer

**Solutions**:

1. **Verify binary is in PATH**:
   ```elisp
   M-: (executable-find "perl-lsp")
   ```

2. **Check LSP session**:
   ```elisp
   M-x lsp-describe-session
   ```

3. **Check LSP logs**:
   ```elisp
   M-x lsp-workspace-show-log
   ```

4. **Test server manually**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

**Symptoms**: No errors shown for invalid code

**Solutions**:

1. **Check file type**:
   ```elisp
   M-: major-mode
   ```
   Should output: `perl-mode` or `cperl-mode`

2. **Set file type manually**:
   ```elisp
   M-x perl-mode
   ```

3. **Verify diagnostics enabled**:
   ```elisp
   M-: lsp-diagnostics-enabled-p
   ```

### Slow Performance

**Symptoms**: Lag when typing, slow completions

**Solutions**:

1. **Reduce result caps**:
   ```elisp
   (setq-default eglot-workspace-configuration
     '((perl
        (limits
         (workspaceSymbolCap . 100)
         (referencesCap . 200)
         (completionCap . 50)))))
   ```

2. **Disable system @INC**:
   ```elisp
   (setq-default eglot-workspace-configuration
     '((perl
        (workspace
         (useSystemInc . :json-false)))))
   ```

3. **Reduce resolution timeout**:
   ```elisp
   (setq-default eglot-workspace-configuration
     '((perl
        (workspace
         (resolutionTimeout . 25)))))
   ```

### Module Resolution Issues

**Symptoms**: Can't find modules, go-to-definition fails

**Solutions**:

1. **Check include paths**:
   ```elisp
   (setq-default eglot-workspace-configuration
     '((perl
        (workspace
         (includePaths . ["lib" "." "local/lib/perl5" "vendor/lib"])))))
   ```

2. **Verify module exists**:
   ```bash
   perl -e 'use Module::Name;'
   ```

3. **Check workspace root**:
   ```elisp
   M-: eglot-current-workspace
   ```

### Formatting Not Working

**Symptoms**: Format command does nothing or errors

**Solutions**:

1. **Install perltidy**:
   ```bash
   # macOS
   brew install perltidy

   # Ubuntu/Debian
   sudo apt-get install perltidy

   # CentOS/RHEL
   sudo yum install perl-Perl-Tidy
   ```

2. **Check perltidy works**:
   ```bash
   perltidy --version
   ```

3. **Verify formatting enabled**:
   ```elisp
   M-x lsp-format-buffer
   ```

---

## Advanced Configuration

### Multi-Root Workspace

For workspaces with multiple folders:

```elisp
(use-package projectile
  :ensure t
  :config
  (projectile-mode +1))

(use-package lsp-mode
  :config
  (setq lsp-auto-guess-root t
        lsp-project-folder-whitelist '("~/projects/")))
```

### Environment Variables

Set environment variables for the LSP server:

```elisp
(lsp-register-client
 (make-lsp-client
  :new-connection (lsp-stdio-connection
                   (lambda ()
                     (list (executable-find "perl-lsp")
                           "--stdio")))
  :environment-fn
  (lambda ()
    `(("PERL5LIB" . ,(concat (projectile-project-root) "lib"))
      ("PERL_MB_OPT" . ,(concat "--install_base " (projectile-project-root) "local"))))))
```

### Custom Handlers

Override default LSP handlers:

```elisp
(lsp-register-client
 (make-lsp-client
  :server-id 'perl-lsp
  :handlers
  (list
   (cons 'textDocument/hover
         (lambda (&rest _)
           (lsp--make-response
            :result
            (lsp-make-hover-contents
             :kind "markdown"
             :value "Custom hover text")))))))
```

### Diagnostic Configuration

Customize diagnostic display:

```elisp
(setq lsp-ui-sideline-show-diagnostics t
      lsp-ui-sideline-diagnostics-max-lines 10
      lsp-modeline-diagnostics-enable t
      lsp-diagnostics-modeline-scope :workspace)

;; Custom diagnostic faces
(custom-set-faces
 '(lsp-face-highlight-read ((t (:background "#3a3a3a"))))
 '(lsp-face-highlight-write ((t (:background "#2a2a3a"))))
 '(lsp-face-highlight-textual ((t (:background "#3a3a3a")))))
```

### Auto-Commands

Set up auto-commands for automatic actions:

```elisp
;; Format on save
(add-hook 'before-save-hook
          (lambda ()
            (when (derived-mode-p 'perl-mode 'cperl-mode)
              (lsp-format-buffer))))

;; Auto-start LSP for Perl files
(add-hook 'perl-mode-hook #'lsp-deferred)
(add-hook 'cperl-mode-hook #'lsp-deferred)
```

### Performance Tuning

For large workspaces, adjust performance settings:

```elisp
(setq-default eglot-workspace-configuration
  '((perl
     (limits
      (workspaceSymbolCap . 100)
      (referencesCap . 200)
      (completionCap . 50)
      (astCacheMaxEntries . 50)
      (maxIndexedFiles . 5000)
      (maxTotalSymbols . 250000)
      (workspaceScanDeadlineMs . 20000)
      (referenceSearchDeadlineMs . 1500))
     (workspace
      (resolutionTimeout . 25)))))

(setq lsp-idle-delay 0.5
      lsp-completion-idle-delay 0.2
      lsp-enable-file-watchers t
      lsp-file-watch-threshold 1000)
```

### Debug Logging

Enable detailed logging for troubleshooting:

```elisp
(setq lsp-log-io t
      lsp-print-performance t
      lsp-trace nil)

;; View logs
M-x lsp-workspace-show-log
```

---

## Complete Example Configuration

Here's a comprehensive example configuration:

```elisp
;; init.el

;; Use package.el
(require 'package)
(add-to-list 'package-archives '("melpa" . "https://melpa.org/packages/") t)
(package-initialize)

;; Use use-package if not installed
(unless (package-installed-p 'use-package)
  (package-install 'use-package))
(require 'use-package)

;; lsp-mode configuration
(use-package lsp-mode
  :ensure t
  :hook ((cperl-mode . lsp-deferred)
         (perl-mode . lsp-deferred))
  :commands lsp
  :init
  (setq lsp-keymap-prefix "C-c l"
        lsp-auto-guess-root t
        lsp-prefer-flymake nil
        lsp-idle-delay 0.5
        lsp-completion-idle-delay 0.2)
  :config
  ;; Register perl-lsp
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perl-lsp" "--stdio"))
    :major-modes '(cperl-mode perl-mode)
    :priority -1
    :server-id 'perl-lsp
    :initialization-options
    '((perl
       (workspace
        (includePaths . ["lib" "." "local/lib/perl5"])
        (useSystemInc . :json-false)
        (resolutionTimeout . 50))
       (inlayHints
        (enabled . t)
        (parameterHints . t)
        (typeHints . t)
        (maxLength . 30))
       (limits
        (workspaceSymbolCap . 200)
        (referencesCap . 500)
        (completionCap . 100)))))))

  ;; Custom keybindings
  (define-key lsp-mode-map (kbd "C-c l") lsp-command-map))

;; lsp-ui for enhanced UI
(use-package lsp-ui
  :ensure t
  :hook (lsp-mode . lsp-ui-mode)
  :config
  (setq lsp-ui-doc-enable t
        lsp-ui-doc-show-with-cursor t
        lsp-ui-sideline-enable t
        lsp-ui-sideline-show-code-actions t
        lsp-ui-sideline-show-diagnostics t))

;; Company for completion
(use-package company
  :ensure t
  :hook (lsp-mode . company-mode)
  :config
  (setq company-minimum-prefix-length 1
        company-tooltip-align-annotations t
        company-idle-delay 0.2
        company-show-numbers t))

;; Company-lsp for LSP completion
(use-package company-lsp
  :ensure t
  :after company
  :config
  (push 'company-lsp company-backends))

;; Keybindings
(with-eval-after-load 'perl-mode
  (define-key perl-mode-map (kbd "M-.") 'xref-find-definitions)
  (define-key perl-mode-map (kbd "M-?") 'xref-find-references)
  (define-key perl-mode-map (kbd "C-c r") 'lsp-rename)
  (define-key perl-mode-map (kbd "C-c a") 'lsp-execute-code-action)
  (define-key perl-mode-map (kbd "C-c f") 'lsp-format-buffer))

(with-eval-after-load 'cperl-mode
  (define-key cperl-mode-map (kbd "M-.") 'xref-find-definitions)
  (define-key cperl-mode-map (kbd "M-?") 'xref-find-references)
  (define-key cperl-mode-map (kbd "C-c r") 'lsp-rename)
  (define-key cperl-mode-map (kbd "C-c a") 'lsp-execute-code-action)
  (define-key cperl-mode-map (kbd "C-c f") 'lsp-format-buffer))

;; Format on save
(add-hook 'before-save-hook
          (lambda ()
            (when (derived-mode-p 'perl-mode 'cperl-mode)
              (lsp-format-buffer))))
```

---

## See Also

- [Getting Started](../GETTING_STARTED.md) - Quick start guide
- [Configuration Reference](../CONFIG.md) - Complete configuration options
- [Troubleshooting Guide](../TROUBLESHOOTING.md) - Common issues and solutions
- [Performance Tuning](../PERFORMANCE_TUNING.md) - Performance optimization guide
- [Editor Setup](../EDITOR_SETUP.md) - Other editor configurations
- [lsp-mode Documentation](https://emacs-lsp.github.io/lsp-mode/)
- [eglot Documentation](https://github.com/joaotavora/eglot)
