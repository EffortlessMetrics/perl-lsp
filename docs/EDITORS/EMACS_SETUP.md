# Emacs Setup Guide for perl-lsp

This guide shows how to use `perllsp` from Emacs.

## Recommended Support Posture

- **Primary path:** Eglot, especially on Emacs 29 or later
- **Alternative path:** `lsp-mode`, for users already using that stack

Both clients launch the same server command:

```bash
perllsp --stdio
```

## Prerequisites

- Emacs 29 or later recommended
- `perllsp` installed and available to Emacs
- A Perl project opened from the project root

Emacs 29 includes Eglot. If you use Emacs 28 or older, install Eglot separately
or use `lsp-mode`.

Install `perllsp` using the project installation guide or README.

Do not install the unrelated crates.io package named `perl-lsp`; that package
name belongs to a different project. Use:

```bash
cargo install perllsp
```

Verify the server before changing Emacs configuration:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Perl File Modes

Emacs has built-in `perl-mode` and `cperl-mode`.

`perl-ts-mode` is optional and third-party. Do not include it in your hooks unless
you have installed a package that provides it.

If files such as `.t`, `.psgi`, `.cgi`, or `.fcgi` are not detected as Perl, add
file associations:

```elisp
(add-to-list 'auto-mode-alist '("\\.t\\'" . perl-mode))
(add-to-list 'auto-mode-alist '("\\.psgi\\'" . perl-mode))
(add-to-list 'auto-mode-alist '("\\.cgi\\'" . perl-mode))
(add-to-list 'auto-mode-alist '("\\.fcgi\\'" . perl-mode))
```

## 1. Minimal Eglot Setup

For Emacs 29+, add this to your Emacs config:

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

If you have installed `perl-ts-mode`, add it explicitly:

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((perl-ts-mode :language-id "perl")
                 . ("perllsp" "--stdio"))))

(add-hook 'perl-ts-mode-hook #'eglot-ensure)
```

Then:

1. Restart Emacs.
2. Open a Perl file such as `lib/My/Module.pm`, `script/app.pl`, or `t/basic.t`.
3. Confirm the mode line shows `[eglot:PROJECT]`.
4. Introduce a temporary syntax error.
5. Confirm Flymake diagnostics appear.
6. Remove the syntax error after testing.

You can also check attachment with:

```elisp
M-: (eglot-managed-p)
```

## Useful Eglot Commands

| Action                  | Command                                |
| ----------------------- | -------------------------------------- |
| Start / reconnect Eglot | `M-x eglot`                            |
| Restart server          | `M-x eglot-reconnect`                  |
| Go to definition        | `M-.` / `M-x xref-find-definitions`    |
| Find references         | `M-?` / `M-x xref-find-references`     |
| Rename symbol           | `M-x eglot-rename`                     |
| Code actions            | `M-x eglot-code-actions`               |
| Format buffer           | `M-x eglot-format-buffer`              |
| Toggle inlay hints      | `M-x eglot-inlay-hints-mode`           |
| Buffer diagnostics      | `M-x flymake-show-buffer-diagnostics`  |
| Project diagnostics     | `M-x flymake-show-project-diagnostics` |
| Protocol log            | `M-x eglot-events-buffer`              |
| Server stderr           | `M-x eglot-stderr-buffer`              |

## Optional: Eglot Initialization Options

Prefer `.perl-lsp.toml` for settings shared across editors. Use Eglot
initialization options only for Emacs-specific startup behavior.

```elisp
(use-package eglot
  :ensure nil
  :hook ((perl-mode . eglot-ensure)
         (cperl-mode . eglot-ensure))
  :config
  (add-to-list 'eglot-server-programs
               '(((perl-mode :language-id "perl")
                  (cperl-mode :language-id "perl"))
                 . ("perllsp" "--stdio"
                    :initializationOptions
                    (:perl
                     (:workspace
                      (:includePaths ["lib" "." "local/lib/perl5"]
                       :useSystemInc :json-false
                       :resolutionTimeout 50)
                      :inlayHints
                      (:enabled t
                       :parameterHints t
                       :typeHints t)))))))
```

## 2. Project Configuration

For team-shared settings, prefer `.perl-lsp.toml` at the repository root.

```toml
[perl]
include_paths = ["lib", ".", "local/lib/perl5", "vendor/lib"]

[diagnostics]
perlcritic = true
perlcritic_severity = 3

[features]
inlay_hints = true
```

If your project only needs the built-in defaults, omit `include_paths`. The
built-in include paths are `lib`, `.`, and `local/lib/perl5`.

## 3. lsp-mode Alternative

Use this path if you already prefer `lsp-mode`.

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

If using `perl-ts-mode`:

```elisp
(with-eval-after-load 'lsp-mode
  (add-to-list 'lsp-language-id-configuration '(perl-ts-mode . "perl")))

(add-hook 'perl-ts-mode-hook #'lsp-deferred)
```

Optional initialization options:

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
    :server-id 'perllsp
    :initialization-options
    '(:perl
      (:workspace
       (:includePaths ["lib" "." "local/lib/perl5"]
        :useSystemInc :json-false
        :resolutionTimeout 50)
       :inlayHints
       (:enabled t
        :parameterHints t
        :typeHints t))))))
```

Keep optional packages such as `lsp-ui`, `company`, `corfu`, `cape`,
`consult`, `vertico`, `orderless`, or modeline integrations layered on only after
base connectivity works.

## 4. Verify It Is Running

### Eglot

1. Open a Perl file.
2. Confirm `[eglot:PROJECT]` appears in the mode line.
3. Run:

   ```elisp
   M-: (eglot-managed-p)
   ```

4. Check diagnostics:

   ```elisp
   M-x flymake-show-buffer-diagnostics
   ```

5. Check logs if needed:

   ```elisp
   M-x eglot-events-buffer
   M-x eglot-stderr-buffer
   ```

### lsp-mode

1. Open a Perl file.
2. Run:

   ```elisp
   M-x lsp-describe-session
   ```

3. Check logs:

   ```elisp
   M-x lsp-workspace-show-log
   ```

## 5. Troubleshooting

### Emacs cannot find `perllsp`

Check inside Emacs:

```elisp
M-: (executable-find "perllsp")
```

It should return a path.

Check from a shell:

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

On Windows PowerShell:

```powershell
where perllsp
perllsp --version
perllsp --health
perllsp --info
```

If GUI Emacs does not inherit your shell `PATH`, either fix Emacs `exec-path` or
use an absolute path:

```elisp
(add-to-list 'eglot-server-programs
             '(((perl-mode :language-id "perl")
                (cperl-mode :language-id "perl"))
               . ("/absolute/path/to/perllsp" "--stdio")))
```

### Emacs starts the wrong Perl language server

`lsp-mode` has existing Perl clients for other Perl language servers. If another
Perl server starts instead of `perllsp`, raise the custom client's priority,
disable the other Perl clients, or remove the other server binary from `PATH`.

### No diagnostics

Check the active major mode:

```elisp
M-: major-mode
```

Expected values include:

```elisp
perl-mode
cperl-mode
perl-ts-mode
```

If the file is not in a Perl mode, add an `auto-mode-alist` entry for the file
extension.

Then check the server outside Emacs:

```bash
perllsp --check path/to/file.pl
```

### Module resolution issues

Prefer `.perl-lsp.toml` for project-wide include paths:

```toml
[perl]
include_paths = ["lib", ".", "local/lib/perl5", "vendor/lib"]
```

Or pass Emacs-specific startup options through Eglot `:initializationOptions` or
`lsp-mode` `:initialization-options`.

### Formatting does not work

Native formatting does not require `perltidy`. Check the Eglot or lsp-mode
server log for native formatting diagnostics. Install `perltidy` only when
using explicit external formatting compatibility mode.

Then try:

```elisp
M-x eglot-format-buffer
```

or, in `lsp-mode`:

```elisp
M-x lsp-format-buffer
```

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input
from Emacs. Use these manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

For server-side behavior and configuration details, see:

- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [Editor Setup](../how-to/EDITOR_SETUP.md)
