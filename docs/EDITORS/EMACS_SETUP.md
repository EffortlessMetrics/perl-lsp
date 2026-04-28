# Emacs Setup Guide for perl-lsp

This guide shows how to use `perllsp` from Emacs with a fast first-run path.

## Official support posture

- **Primary path (Emacs 29+)**: `eglot`
- **Alternative path**: `lsp-mode` (for users already invested in that stack)

Both clients launch the same server command:

```bash
perllsp --stdio
```

## Prerequisites

- Emacs 29+ recommended
- `perllsp` installed and available on your `PATH`

Install `perllsp` using the same methods described in:

- [README Quick Start](../../README.md)
- [Getting Started](../tutorials/GETTING_STARTED.md)

> Do not use `cargo install perl-lsp` (different crates.io package name).

Verify the server before changing Emacs configuration:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Perl file modes

Emacs includes `perl-mode` and `cperl-mode`.

`perl-ts-mode` is third-party (not bundled in Emacs core); only add it to hooks
if you have installed a package that provides it.

If file types such as `.t`, `.psgi`, `.cgi`, or `.fcgi` are not detected as
Perl, add explicit associations:

```elisp
(add-to-list 'auto-mode-alist '("\\.t\\'" . perl-mode))
(add-to-list 'auto-mode-alist '("\\.psgi\\'" . perl-mode))
(add-to-list 'auto-mode-alist '("\\.cgi\\'" . perl-mode))
(add-to-list 'auto-mode-alist '("\\.fcgi\\'" . perl-mode))
```

## 1) Minimal setup (recommended)

Copy this into your Emacs config:

```elisp
(use-package eglot
  :ensure nil
  :hook ((perl-mode . eglot-ensure)
         (cperl-mode . eglot-ensure))
  :config
  (add-to-list 'eglot-server-programs
               '(((perl-mode :language-id "perl")
                  (cperl-mode :language-id "perl"))
                 . ("perllsp" "--stdio"))))
```

If you use `perl-ts-mode`, add it explicitly:

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((perl-ts-mode :language-id "perl")
                 . ("perllsp" "--stdio"))))

(add-hook 'perl-ts-mode-hook #'eglot-ensure)
```

Then:

1. Restart Emacs.
2. Open a Perl file (for example `.pm`, `.pl`, or `.t`).
3. Confirm the mode line shows `[eglot:PROJECT]`.
4. Introduce a temporary syntax error and confirm Flymake diagnostics appear.

You can also verify attachment directly:

```elisp
M-: (eglot-managed-p)
```

### Useful Eglot commands

- Start/reconnect: `M-x eglot`
- Restart server: `M-x eglot-reconnect`
- Go to definition: `M-.`
- Find references: `M-?`
- Rename symbol: `M-x eglot-rename`
- Code actions: `M-x eglot-code-actions`
- Format buffer: `M-x eglot-format-buffer`
- Buffer diagnostics: `M-x flymake-show-buffer-diagnostics`
- Protocol log: `M-x eglot-events-buffer`
- Server stderr: `M-x eglot-stderr-buffer`

## 2) Team/project configuration (preferred over large Emacs Lisp)

Keep project-wide defaults in `.perl-lsp.toml` at repository root.

Example:

```toml
[perl]
include_paths = ["lib", ".", "local/lib/perl5", "vendor/lib"]

[diagnostics]
perlcritic = true
perlcritic_severity = 3

[features]
inlay_hints = true
```

If your project only needs defaults, `include_paths` can be omitted. The built-in
search paths already include `lib`, `.`, and `local/lib/perl5`.

This keeps startup config thin and makes settings portable across editors.

For full configuration reference, see [docs/reference/CONFIG.md](../reference/CONFIG.md).

## 3) lsp-mode alternative

If you prefer `lsp-mode`, use this setup:

```elisp
(use-package lsp-mode
  :commands (lsp lsp-deferred)
  :hook ((perl-mode . lsp-deferred)
         (cperl-mode . lsp-deferred))
  :config
  (add-to-list 'lsp-language-id-configuration '(perl-mode . "perl"))
  (add-to-list 'lsp-language-id-configuration '(cperl-mode . "perl"))

  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perllsp" "--stdio"))
    :activation-fn (lsp-activate-on "perl")
    :major-modes '(perl-mode cperl-mode)
    :priority 1
    :server-id 'perllsp)))
```

If you use `perl-ts-mode`:

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(perl-ts-mode . "perl")))

(add-hook 'perl-ts-mode-hook #'lsp-deferred)
```

Keep optional packages (`lsp-ui`, custom completion stacks, extra modeline integrations)
layered on only after base connectivity works.

## 4) Troubleshooting quick checks

Run these in order:

1. Binary resolution:

   ```elisp
   M-: (executable-find "perllsp")
   ```

2. Verify server outside Emacs:

   ```bash
   perllsp --version
   perllsp --health
   perllsp --info
   ```

3. Verify active major mode:

   ```elisp
   M-: major-mode
   ```

   Expected: `perl-mode`, `cperl-mode`, or an installed `perl-ts-mode`.

4. Check client attachment:

   - `eglot`: confirm `[eglot:PROJECT]` in the mode line and run `M-: (eglot-managed-p)`
   - `lsp-mode`: `M-x lsp-describe-session`

5. Check logs:

   - `eglot`: `M-x eglot-events-buffer`, `M-x eglot-stderr-buffer`
   - `lsp-mode`: `M-x lsp-workspace-show-log`

6. Validate server behavior without editor transport:

   ```bash
   perllsp --check path/to/file.pl
   ```

If `perllsp --stdio` appears to hang when run manually, that is expected.
In stdio mode the server waits for framed LSP JSON-RPC input from the editor.

If the CLI checks pass but Emacs does not attach, see [Troubleshooting](../how-to/TROUBLESHOOTING.md).

## 5) Scope and expectations

This page intentionally focuses on:

- a successful first connection,
- consistent binary naming (`perllsp`), and
- consistent mode coverage (`perl-mode` + `cperl-mode` + optional `perl-ts-mode`).

For broader editor guidance, see [Editor Setup](../how-to/EDITOR_SETUP.md).
