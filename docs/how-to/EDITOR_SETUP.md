# Editor Setup Guide

Use this page after `perllsp` is installed and visible on your `PATH`. If you
still need the binary, start with [INSTALLATION.md](INSTALLATION.md).

If the server starts but the editor does not behave correctly, see
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).

## What Every Editor Needs

- `perllsp` available on `PATH`
- a workspace root that contains your Perl files
- a command that starts the server with stdio, usually `perllsp --stdio`

Verify the install before debugging editor settings:

```bash
perllsp --version
perllsp --health
```

## Pick Your Editor

| Editor | Fast path | Detailed guide |
| --- | --- | --- |
| VS Code | install the extension or point it at `perllsp --stdio` | [docs/EDITORS/VS_CODE_SETUP.md](../EDITORS/VS_CODE_SETUP.md) |
| Neovim | configure `cmd = { "perllsp", "--stdio" }` | [docs/EDITORS/NEOVIM_SETUP.md](../EDITORS/NEOVIM_SETUP.md) |
| Emacs | use `lsp-mode` or `eglot` with `perllsp --stdio` | [docs/EDITORS/EMACS_SETUP.md](../EDITORS/EMACS_SETUP.md) |
| Helix | add a `perllsp` language server entry | [docs/EDITORS/HELIX_SETUP.md](../EDITORS/HELIX_SETUP.md) |
| Sublime Text | register `perllsp` in the LSP package settings | [docs/EDITORS/SUBLIME_SETUP.md](../EDITORS/SUBLIME_SETUP.md) |
| Notepad++ | configure your LSP plugin to run `perllsp --stdio` | [docs/EDITORS/NOTEPAD_PLUS_PLUS_SETUP.md](../EDITORS/NOTEPAD_PLUS_PLUS_SETUP.md) |

## Minimal Configurations

### VS Code

The repo-maintained extension is the easiest route. If you prefer a manual
configuration, set the command to `perllsp --stdio` and keep the workspace
root pointed at the project root.

### Neovim

```lua
require('lspconfig').perl_lsp.setup({
  cmd = { 'perllsp', '--stdio' },
  filetypes = { 'perl' },
})
```

### Emacs

Use `lsp-mode` or `eglot` with the same `perllsp --stdio` command. The
editor-specific guide has the full snippets for both.

### Helix

```toml
[[language-server.perl-lsp]]
command = "perllsp"
args = ["--stdio"]
```

### Sublime Text

Register a client whose command is `["perllsp", "--stdio"]` and scope it to
Perl source files.

### Notepad++

Using your Notepad++ LSP plugin (for example NppLspClient), register a Perl
server command as `["perllsp", "--stdio"]` and map Perl file types (`.pl`,
`.pm`, `.t`, etc.) to language id `perl`.

## When Setup Fails

- If the server is not found, re-run `perllsp --version` in a shell and fix
  `PATH` first.
- If the server starts but the editor stays idle, check the editor's LSP log
  and confirm the workspace root is correct.
- If completions or diagnostics are missing, move to
  [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for the next steps.
