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
| Trae (ByteDance) | install the VS Code-compatible extension or set command to `perllsp --stdio` | [docs/EDITORS/TRAE_SETUP.md](../EDITORS/TRAE_SETUP.md) |
| Neovim | configure `cmd = { "perllsp", "--stdio" }` | [docs/EDITORS/NEOVIM_SETUP.md](../EDITORS/NEOVIM_SETUP.md) |
| Emacs | use `lsp-mode` or `eglot` with `perllsp --stdio` | [docs/EDITORS/EMACS_SETUP.md](../EDITORS/EMACS_SETUP.md) |
| Helix | add a `perllsp` language server entry | [docs/EDITORS/HELIX_SETUP.md](../EDITORS/HELIX_SETUP.md) |
| Zed | install a Perl extension, then optionally point at `perllsp` | [docs/EDITORS/ZED_SETUP.md](../EDITORS/ZED_SETUP.md) |
| Sublime Text | register `perllsp` in the LSP package settings | [docs/EDITORS/SUBLIME_SETUP.md](../EDITORS/SUBLIME_SETUP.md) |
| Amazon Kiro | register a Perl LSP client using `perllsp --stdio` | [docs/EDITORS/KIRO_SETUP.md](../EDITORS/KIRO_SETUP.md) |
| Eclipse | configure a Language Server entry for `perllsp --stdio` | [docs/EDITORS/ECLIPSE_SETUP.md](../EDITORS/ECLIPSE_SETUP.md) |

## Minimal Configurations

### VS Code

The repo-maintained extension is the easiest route. If you prefer a manual
configuration, set the command to `perllsp --stdio` and keep the workspace
root pointed at the project root.

### Trae (ByteDance)

Trae is VS Code-compatible, so the same extension and settings model applies.
Install the `EffortlessMetrics.perl-lsp-rs` extension from Trae's Extensions
panel, or configure a generic language server command as `perllsp --stdio`.

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
[[language]]
name = "perl"
language-servers = ["perllsp"]

[language-server.perllsp]
command = "perllsp"
args = ["--stdio"]
```

### Zed

Zed requires a Perl extension that registers `perllsp`. Once installed, you
can override the binary path via `settings.json`:

```json
{
  "lsp": {
    "perl-lsp": {
      "binary": {
        "path": "/usr/local/bin/perllsp",
        "arguments": ["--stdio"]
      }
    }
  }
}
```

See [docs/EDITORS/ZED_SETUP.md](../EDITORS/ZED_SETUP.md) for full setup details.


### Sublime Text

Register a client whose command is `["perllsp", "--stdio"]` and scope it to
Perl source files.

### Amazon Kiro

Register a Perl language-server client that launches `perllsp --stdio`, then
restart the client after changing workspace settings or include paths.

### Eclipse

In `Preferences` → `Language Servers`, add a server command `perllsp` with
argument `--stdio`, then associate it with Perl file patterns like `*.pl`,
`*.pm`, and `*.t`.

## When Setup Fails

- If the server is not found, re-run `perllsp --version` in a shell and fix
  `PATH` first.
- If the server starts but the editor stays idle, check the editor's LSP log
  and confirm the workspace root is correct.
- If completions or diagnostics are missing, move to
  [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for the next steps.
