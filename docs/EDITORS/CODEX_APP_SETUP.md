# OpenAI Codex App Setup Guide for perl-lsp

This guide covers using `perllsp` with the OpenAI Codex app.

Codex app is not a traditional code editor or generic LSP client. It does not
currently expose a documented custom language-server registration flow for live
LSP features such as editor diagnostics, hover, go-to-definition, or
references.

Use this guide when you want Codex to run `perllsp` checks and use the results
while working on a Perl project.

For live editor LSP features while using Codex in an IDE, install the Codex IDE
Extension and the `EffortlessMetrics.perl-lsp-rs` Perl LSP extension in the
same VS Code-compatible editor. See
[docs/EDITORS/VS_CODE_SETUP.md](./VS_CODE_SETUP.md).

## Prerequisites

- OpenAI Codex app installed
- A Perl project opened in Codex
- `perllsp` installed and available to the shell environment used by Codex

Verify the server first:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Recommended Project Configuration

Put shared `perl-lsp` behavior in `.perl-lsp.toml` at the repository root:

```toml
[perl]
include_paths = ["lib", ".", "local/lib/perl5", "vendor/lib"]

[features]
inlay_hints = true

[diagnostics]
perlcritic = false
perlcritic_severity = 3
```

If your project only needs the built-in include paths, omit `include_paths`.
The built-in defaults are `lib`, `.`, and `local/lib/perl5`.

## Use perllsp from Codex

Codex can run commands in the project through its integrated terminal or
project actions.

Useful commands:

```bash
perllsp --check path/to/file.pl
perllsp --check-project .
perllsp --health
perllsp --info
```

Example prompts:

```text
Run `perllsp --check-project .` and summarize any Perl parse errors.
```

```text
Run `perllsp --check lib/My/Module.pm`, explain the diagnostic, and propose a minimal fix.
```

```text
After editing this Perl file, run `perllsp --check path/to/file.pl` and verify the parse error is gone.
```

## Optional: Add a Codex Project Action

In Codex app settings, add a project action that runs:

```bash
perllsp --check-project .
```

Suggested action name:

```text
Check Perl with perllsp
```

## Do Not Register perllsp as MCP

Do not add this under Codex `[mcp_servers]`:

```toml
[mcp_servers.perl-lsp]
command = "perllsp"
args = ["--stdio"]
```

`perllsp --stdio` speaks the Language Server Protocol, not the Model Context
Protocol. Codex MCP configuration is for MCP servers that expose tools and
context to Codex.

## Live LSP Features While Using Codex

For live editor features such as diagnostics, hover, completion, semantic
tokens, go-to-definition, references, rename, and formatting:

1. Use a VS Code-compatible editor.
2. Install the OpenAI Codex IDE Extension.
3. Install the Perl LSP extension:

   ```text
   EffortlessMetrics.perl-lsp-rs
   ```

The Perl LSP extension can auto-download `perllsp`. For manual or offline
setups, set:

```json
{
  "perl-lsp.serverPath": "/absolute/path/to/perllsp",
  "perl-lsp.autoDownload": false
}
```

See [docs/EDITORS/VS_CODE_SETUP.md](./VS_CODE_SETUP.md) for full setup.

## Troubleshooting

### Codex cannot find `perllsp`

Check from the same shell environment Codex uses:

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

If Codex was launched from a GUI, it may not inherit the same `PATH` as your
terminal. Use absolute paths in project actions if needed.

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input
from an editor. For manual checks, use:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
perllsp --check-project .
```

### Codex app does not show hover, references, or go-to-definition

That is expected unless you are using a separate editor with an LSP client.
Codex app can run `perllsp` commands and reason over their output, but live LSP
UI belongs in an editor such as VS Code, Neovim, Emacs, Sublime, Helix, or Zed.

For server-side behavior and configuration details, see:

- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [Editor Setup](../how-to/EDITOR_SETUP.md)
