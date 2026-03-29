# Editor Setup Guide

This guide assumes `perl-lsp` is already installed and available on your
`PATH`. If it is not, start with [INSTALLATION.md](INSTALLATION.md).

## Pick Your Editor

- [VS Code](#vs-code)
- [Neovim](#neovim)
- [Emacs](#emacs)
- [Helix](#helix)
- [Sublime Text](#sublime-text)

## VS Code

Install the official extension:

```bash
code --install-extension EffortlessMetrics.perl-lsp-rs
```

The extension auto-downloads the server binary. If you want to use a local
binary instead, set `perl-lsp.serverPath` in your settings.

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.serverPath": "",
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true
}
```

## Neovim

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern('.git', 'Makefile.PL', 'cpanfile', 'dist.ini'),
      single_file_support = true,
    },
  }
end

lspconfig.perl_lsp.setup({})
```

## Emacs

```elisp
(add-to-list 'eglot-server-programs
             '((cperl-mode perl-mode) . ("perl-lsp" "--stdio")))
```

If you use `lsp-mode`, register the same command with `lsp-register-client`.

## Helix

```toml
[[language]]
name = "perl"
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]
```

## Sublime Text

Configure the LSP package to run:

```text
perl-lsp --stdio
```

## What To Check

After editor setup, open a Perl file and confirm:

- the server starts without errors
- diagnostics appear for obvious syntax issues
- hover and completion work on a small test file

If that does not happen, move to [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
