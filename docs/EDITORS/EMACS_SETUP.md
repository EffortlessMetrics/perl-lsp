# Emacs Setup Guide for perl-lsp

This guide gives you a fast, reliable Emacs setup for `perllsp` with a minimal first-run path.

## Official support posture

- **Primary path (Emacs 29+)**: `eglot`
- **Alternative path**: `lsp-mode` (for users who prefer that stack)

Both clients are supported with the same server command:

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

## 1) Minimal setup (recommended)

Copy this into your Emacs config:

```elisp
(use-package eglot
  :hook ((perl-mode . eglot-ensure)
         (cperl-mode . eglot-ensure)
         (perl-ts-mode . eglot-ensure))
  :config
  (add-to-list 'eglot-server-programs
               '((perl-mode cperl-mode perl-ts-mode) . ("perllsp" "--stdio"))))
```

Then:

1. Restart Emacs.
2. Open a `.pl` or `.pm` file.
3. Confirm `eglot` is attached (`M-x eglot`).

### Minimal key commands

- Go to definition: `M-.`
- Find references: `M-?`
- Rename symbol: `M-x eglot-rename`
- Code actions: `M-x eglot-code-actions`
- Format buffer: `M-x eglot-format-buffer`

With `eglot`, diagnostics are provided through Flymake in standard Emacs UI.

## 2) Team/project configuration (preferred over large Emacs Lisp)

Keep project-wide defaults in `.perl-lsp.toml` at repository root.

Example:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
enabled = true

[format]
enabled = true

[perlcritic]
enabled = true
```

This keeps editor startup config thin and makes settings portable across VS Code, Neovim, Emacs, and other clients.

For full configuration reference, see [docs/reference/CONFIG.md](../reference/CONFIG.md).

## 3) lsp-mode alternative

If you prefer `lsp-mode`, use this minimal config:

```elisp
(use-package lsp-mode
  :hook ((perl-mode . lsp-deferred)
         (cperl-mode . lsp-deferred)
         (perl-ts-mode . lsp-deferred))
  :commands lsp
  :config
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perllsp" "--stdio"))
    :major-modes '(perl-mode cperl-mode perl-ts-mode)
    :server-id 'perllsp)))
```

Keep optional packages (`lsp-ui`, custom completion stacks, extra modeline integrations) layered on only after base connectivity works.

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
   ```

3. Verify active major mode:

   ```elisp
   M-: major-mode
   ```

   Expected: `perl-mode`, `cperl-mode`, or `perl-ts-mode`.

4. Check client attachment:

   - `eglot`: `M-x eglot`
   - `lsp-mode`: `M-x lsp-describe-session`

5. Optional low-level server check:

   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perllsp --stdio
   ```

If the CLI checks pass but Emacs does not attach, see [Troubleshooting](../how-to/TROUBLESHOOTING.md).

## 5) Scope and expectations

This page intentionally focuses on:

- a successful first connection,
- consistent binary naming (`perllsp`), and
- consistent mode coverage (`perl-mode` + `cperl-mode` + `perl-ts-mode`).

For broader editor guidance, see [Editor Setup](../how-to/EDITOR_SETUP.md).
