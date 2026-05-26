# Editor Setup Guide

Use this page after choosing an install path.

For VS Code-compatible editors, the extension can download `perllsp`
automatically. For generic LSP clients, install `perllsp` first and make sure
it is visible on your `PATH`. If you still need the binary, start with
[INSTALLATION.md](INSTALLATION.md).

Current public install artifacts are public alpha. Verify `perllsp --version`
and `perllsp --health` before changing editor settings.

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
| Cursor | install the VS Code-compatible extension and configure it with the `perl-lsp.*` settings namespace | [docs/EDITORS/CURSOR_SETUP.md](../EDITORS/CURSOR_SETUP.md) |
| Trae (ByteDance) | install the VS Code-compatible extension or set command to `perllsp --stdio` | [docs/EDITORS/TRAE_SETUP.md](../EDITORS/TRAE_SETUP.md) |
| IntelliJ IDEA | install the LSP4IJ plugin and register `perllsp --stdio` as a Raw Command server | [docs/EDITORS/INTELLIJ_IDEA_SETUP.md](../EDITORS/INTELLIJ_IDEA_SETUP.md) |
| Neovim | define a custom `perllsp` config with `vim.lsp.config()` and enable via `vim.lsp.enable()` (legacy `nvim-lspconfig` supported for older Neovim) | [docs/EDITORS/NEOVIM_SETUP.md](../EDITORS/NEOVIM_SETUP.md) |
| Vim | use `vim-lsp` with `perllsp --stdio` | [docs/EDITORS/VIM_SETUP.md](../EDITORS/VIM_SETUP.md) |
| coc.nvim | configure `languageserver.perl-lsp` in `coc-settings.json` to launch `perllsp --stdio`; works in Neovim and Vim when the buffer filetype is `perl` | [docs/EDITORS/COC_NEOVIM_SETUP.md](../EDITORS/COC_NEOVIM_SETUP.md) |
| Emacs | use `lsp-mode` or `eglot` with `perllsp --stdio` | [docs/EDITORS/EMACS_SETUP.md](../EDITORS/EMACS_SETUP.md) |
| Helix | add a `perllsp` language server entry | [docs/EDITORS/HELIX_SETUP.md](../EDITORS/HELIX_SETUP.md) |
| Zed | requires a Zed Perl extension that registers `perllsp`; `settings.json` can override binary and initialization options for that registered server | [docs/EDITORS/ZED_SETUP.md](../EDITORS/ZED_SETUP.md) |
| Sublime Text | register `perllsp` in LSP package settings | [docs/EDITORS/SUBLIME_SETUP.md](../EDITORS/SUBLIME_SETUP.md) |
| Amazon Kiro | register a Perl LSP client using `perllsp --stdio` | [docs/EDITORS/KIRO_SETUP.md](../EDITORS/KIRO_SETUP.md) |
| Claude Code | provide a plugin `.lsp.json` pointing to `perllsp --stdio` | [docs/EDITORS/CLAUDE_CODE_SETUP.md](../EDITORS/CLAUDE_CODE_SETUP.md) |
| Codex CLI | configure an MCP bridge such as `lsp-mcp`; the bridge exposes tools to Codex and launches `perllsp --stdio` internally | [docs/EDITORS/CODEX_CLI_SETUP.md](../EDITORS/CODEX_CLI_SETUP.md) |
| Codex Desktop | add a custom Perl server command `perllsp --stdio` | [docs/EDITORS/CODEX_DESKTOP_SETUP.md](../EDITORS/CODEX_DESKTOP_SETUP.md) |
| OpenCode | configure a custom `perl-lsp` server in `opencode.json` | [docs/EDITORS/OPENCODE_SETUP.md](../EDITORS/OPENCODE_SETUP.md) |

## Minimal Configurations

### VS Code

The repo-maintained extension is the easiest route. If you prefer a manual
configuration, set the command to `perllsp --stdio` and keep the workspace
root pointed at the project root.

### Cursor

Cursor is based on the VS Code codebase. Install the `EffortlessMetrics.perl-lsp-rs`
extension from Cursor's Extensions panel (or via VSIX). Workspace settings use
the same `.vscode/settings.json` format as VS Code.

### Trae (ByteDance)

Trae is VS Code-compatible, so the same extension and settings model applies.
Install the `EffortlessMetrics.perl-lsp-rs` extension from Trae's Extensions
panel, or configure a generic language server command as `perllsp --stdio`.

### Neovim

For Neovim 0.11+:

```lua
vim.lsp.config('perllsp', {
  cmd = { 'perllsp', '--stdio' },
  filetypes = { 'perl' },
  root_markers = {
    '.perl-lsp.toml',
    'Makefile.PL',
    'Build.PL',
    'cpanfile',
    'dist.ini',
    '.git',
  },
  init_options = {
    perl = {
      workspace = {
        includePaths = { 'lib', '.', 'local/lib/perl5' },
      },
    },
  },
})

vim.lsp.enable('perllsp')
```

For latency-focused editing, use the lean profile from
[docs/EDITORS/NEOVIM_SETUP.md](../EDITORS/NEOVIM_SETUP.md). It starts
`perllsp` with `--runtime-mode e2e`, syntax-only diagnostics, zero diagnostic
debounce, disabled eager workspace indexing, and disabled file watchers. That
profile favors responsiveness over full semantic/module/critic/dead-code
diagnostics and does not provide incremental AST reuse.

### Emacs

Use `lsp-mode` or `eglot` with the same `perllsp --stdio` command. The
editor-specific guide has the full snippets for both.

### Vim

Use `vim-lsp` configured to launch `perllsp --stdio`. See
[docs/EDITORS/VIM_SETUP.md](../EDITORS/VIM_SETUP.md) for complete examples.

### coc.nvim

Open coc.nvim settings:

```vim
:CocConfig
```

Add:

```json
{
  "languageserver": {
    "perl-lsp": {
      "command": "perllsp",
      "args": ["--stdio"],
      "filetypes": ["perl"],
      "rootPatterns": [".perl-lsp.toml", "Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"],
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"]
          }
        }
      }
    }
  }
}
```

Check the active filetype with:

```vim
:CocCommand document.echoFiletype
```

It must be `perl`.

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

Zed does not create arbitrary language servers from `settings.json` alone. A
Zed language extension must first register a Perl language server ID, for
example `perl-lsp`.

Once that extension exists, configure the server in Zed settings:

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

The public Zed Perl extension currently registers `perlnavigator-server`, not
`perllsp`, so use a perllsp-capable extension or development extension before
applying the `perl-lsp` settings.

See [docs/EDITORS/ZED_SETUP.md](../EDITORS/ZED_SETUP.md) for full setup details.


### Sublime Text

Install the `LSP` package, then open `Preferences: LSP Server Configurations` and add:

```json
{
  "perl-lsp": {
    "enabled": true,
    "command": ["perllsp", "--stdio"],
    "selector": "source.perl"
  }
}
```

For project-specific server settings, use `.perl-lsp.toml` or add Sublime `initialization_options` under the `perl-lsp` server configuration.

### Amazon Kiro

For Kiro IDE, install the OpenVSX extension `EffortlessMetrics.perl-lsp-rs`.
The extension can auto-download `perllsp`. For Kiro CLI, run `/code init` in
the project root and edit the generated LSP configuration so Perl launches with
`perllsp --stdio`. Verify diagnostics, hover, definition, references, and rename
in your installed Kiro CLI build because Perl uses the custom-LSP path there.

### Claude Code

Create a plugin with `.lsp.json` that maps Perl extensions to a server entry
using `command: "perllsp"` and `args: ["--stdio"]`.

### Codex CLI

Codex CLI uses MCP tools rather than direct LSP server registration. Configure
an LSP-to-MCP bridge such as `lsp-mcp`, then point Codex at the bridge with a
project-local `.codex/config.toml` entry like:

```toml
[mcp_servers.perl_lsp]
command = "lsp-mcp"
args = ["--config", "/absolute/path/to/project/lsp-mcp.toml", "--workspace", "/absolute/path/to/project"]
cwd = "/absolute/path/to/project"
```

Do not register `perllsp --stdio` directly as an MCP server; it speaks LSP, not
MCP. See [docs/EDITORS/CODEX_CLI_SETUP.md](../EDITORS/CODEX_CLI_SETUP.md) for
the full workflow, bridge config, and troubleshooting.

### OpenCode

Create or update `opencode.json` and register a custom LSP server with
`"command": ["perllsp", "--stdio"]` and Perl extensions like `.pl`, `.pm`,
and `.t`. See [docs/EDITORS/OPENCODE_SETUP.md](../EDITORS/OPENCODE_SETUP.md) for a full example.

### IntelliJ IDEA

Install the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin,
then open **File > Settings > Languages & Frameworks > Language Servers** and add:

| Field | Value |
| --- | --- |
| Name | `perl-lsp` |
| Command | `perllsp --stdio` |
| File name patterns | `*.pl`, `*.pm`, `*.t`, `*.psgi`, `*.cgi` |

See [docs/EDITORS/INTELLIJ_IDEA_SETUP.md](../EDITORS/INTELLIJ_IDEA_SETUP.md) for the full workflow,
initialization options, and troubleshooting.

LSP4IJ-shaped clients use standard LSP 3.18 inline completion:
`textDocument.inlineCompletion.dynamicRegistration` selects dynamic
registration for `textDocument/inlineCompletion`; static clients receive
top-level `inlineCompletionProvider`. `experimental.inlineCompletionProvider`
is not used.

## When Setup Fails

- If the server is not found, re-run `perllsp --version` in a shell and fix
  `PATH` first.
- If the server starts but the editor stays idle, check the editor's LSP log
  and confirm the workspace root is correct.
- If completions or diagnostics are missing, move to
  [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for the next steps.


### Codex Desktop

Configure a custom Perl language server process that runs `perllsp --stdio`.
See the dedicated guide for the exact fields and verification steps.
