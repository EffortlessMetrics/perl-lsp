# Editor Setup Guide

Use this page after perl-lsp is installed and visible on your `PATH`. If you
still need the binary, start with [INSTALLATION.md](INSTALLATION.md).

If the server starts but the editor does not behave correctly, see
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## What Every Editor Needs

- `perl-lsp` available on `PATH`
- a workspace root that contains your Perl files
- a command that starts the server with stdio, usually `perl-lsp --stdio`

Verify the install before debugging editor settings:

```bash
perl-lsp --version
perl-lsp --health
```

## Pick Your Editor

| Editor | Fast path | Detailed guide |
| --- | --- | --- |
| VS Code | install the extension or point it at `perl-lsp --stdio` | [docs/EDITORS/VS_CODE_SETUP.md](../EDITORS/VS_CODE_SETUP.md) |
| Neovim | configure `cmd = { "perl-lsp", "--stdio" }` | [docs/EDITORS/NEOVIM_SETUP.md](../EDITORS/NEOVIM_SETUP.md) |
| Emacs | use `lsp-mode` or `eglot` with `perl-lsp --stdio` | [docs/EDITORS/EMACS_SETUP.md](../EDITORS/EMACS_SETUP.md) |
| Helix | add a `perl-lsp` language server entry | [docs/EDITORS/HELIX_SETUP.md](../EDITORS/HELIX_SETUP.md) |
| Sublime Text | register `perl-lsp` in the LSP package settings | [docs/EDITORS/SUBLIME_SETUP.md](../EDITORS/SUBLIME_SETUP.md) |

## Minimal Configurations

### VS Code

The repo-maintained extension is the easiest route. If you prefer a manual
configuration, set the command to `perl-lsp --stdio` and keep the workspace
root pointed at the project root.

### Neovim

```lua
require('lspconfig').perl_lsp.setup({
  cmd = { 'perl-lsp', '--stdio' },
  filetypes = { 'perl' },
})
```

### Emacs

Use `lsp-mode` or `eglot` with the same `perl-lsp --stdio` command. The
editor-specific guide has the full snippets for both.

### Helix

```toml
[[language-server.perl-lsp]]
command = "perl-lsp"
args = ["--stdio"]
```

### Sublime Text

Register a client whose command is `["perl-lsp", "--stdio"]` and scope it to
Perl source files.

## When Setup Fails

- If the server is not found, re-run `perl-lsp --version` in a shell and fix
  `PATH` first.
- If the server starts but the editor stays idle, check the editor's LSP log
  and confirm the workspace root is correct.
- If completions or diagnostics are missing, move to
  [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for the next steps.
