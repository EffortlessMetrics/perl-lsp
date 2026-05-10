# Codex CLI + perl-lsp Setup

This guide shows how to make Codex CLI use `perllsp` through an MCP bridge.

Codex CLI does not register LSP servers directly. Codex calls tools exposed by
Model Context Protocol (MCP) servers. For Perl LSP features, run an MCP bridge
that launches `perllsp --stdio` behind the scenes.

## Prerequisites

- Codex CLI installed and authenticated
- `perllsp` installed and available on `PATH`
- An LSP-to-MCP bridge installed, for example `lsp-mcp`
- A Perl project opened as your Codex working directory

Quick sanity checks:

```bash
perllsp --version
perllsp --health
perllsp --info
codex --version
```

## 1) Install an LSP MCP bridge

One bridge option is `lsp-mcp`:

```bash
cargo install lsp-mcp
```

Verify it is available:

```bash
lsp-mcp --help
```

## 2) Configure lsp-mcp for Perl

Create `lsp-mcp.toml` in your project root:

```toml
[[servers]]
name = "perl"
command = ["perllsp", "--stdio"]
extensions = [".pl", ".PL", ".pm", ".t", ".pod", ".psgi", ".cgi", ".fcgi", ".xs", ".xsi"]
root_markers = [".perl-lsp.toml", "Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"]
language_id = "perl"
```

Validate the bridge config:

```bash
lsp-mcp check --config ./lsp-mcp.toml
```

You can also start the bridge manually for smoke testing:

```bash
lsp-mcp --config ./lsp-mcp.toml --workspace "$PWD"
```

It is normal for this command to keep running; it is waiting for MCP JSON-RPC
input from a client.

## 3) Add the bridge to Codex MCP config

Use either user-level config (`~/.codex/config.toml`) or trusted project-local
config (`.codex/config.toml`).

For a project-specific setup, create or update `.codex/config.toml`:

```toml
[mcp_servers.perl_lsp]
command = "lsp-mcp"
args = [
  "--config",
  "/absolute/path/to/project/lsp-mcp.toml",
  "--workspace",
  "/absolute/path/to/project"
]
cwd = "/absolute/path/to/project"
startup_timeout_sec = 20
tool_timeout_sec = 120
```

Replace `/absolute/path/to/project` with your repository root.

You can also add the server from Codex CLI:

```bash
codex mcp add perl_lsp -- lsp-mcp --config /absolute/path/to/project/lsp-mcp.toml --workspace /absolute/path/to/project
```

## 4) Recommended perl-lsp project config

Prefer `.perl-lsp.toml` for settings shared across editors and agents:

```toml
[perl]
include_paths = ["lib", ".", "local/lib/perl5", "vendor/lib"]

[features]
inlay_hints = true

[diagnostics]
# Native critic diagnostics are enabled by default.
# Set false to disable them.
# perlcritic = true
perlcritic_severity = 3
```

If your project only needs built-in include paths, omit `include_paths`. The
built-in defaults are `lib`, `.`, and `local/lib/perl5`.

## 5) Validate from Codex

Start Codex from the project root:

```bash
codex
```

Inside Codex, run:

```text
/mcp
```

Confirm the `perl_lsp` MCP server is listed and tools are available.

Good test prompts:

```text
Use the Perl LSP MCP tools to show diagnostics for lib/My/Module.pm.
```

```text
Use the Perl LSP MCP tools to show hover information for the symbol at
lib/My/Module.pm line 42 column 7.
```

```text
Use the Perl LSP MCP tools to find references for the symbol at
lib/My/Module.pm line 42 column 7.
```

```text
Use the Perl LSP MCP tools to preview renaming the symbol at
lib/My/Module.pm line 42 column 7 to build_workspace_index. Show me the edits
before applying them.
```

For diagnostics without MCP, you can always run:

```bash
perllsp --check path/to/file.pl
perllsp --check-project .
```

## Do Not Register perllsp Directly as MCP

Do not configure Codex like this:

```toml
[mcp_servers.perl_lsp]
command = "perllsp"
args = ["--stdio"]
```

`perllsp --stdio` speaks the Language Server Protocol. Codex MCP configuration
expects a Model Context Protocol server. Use an MCP bridge such as `lsp-mcp`;
the bridge launches `perllsp --stdio` internally.

## Troubleshooting

### No LSP tools appear in `/mcp`

Check the bridge outside Codex:

```bash
lsp-mcp check --config ./lsp-mcp.toml
lsp-mcp --config ./lsp-mcp.toml --workspace "$PWD"
```

Then restart Codex and run:

```text
/mcp
```

### `perllsp` not found

Check from the same shell used to launch Codex:

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

If needed, use an absolute path in `lsp-mcp.toml`:

```toml
[[servers]]
name = "perl"
command = ["/absolute/path/to/perllsp", "--stdio"]
extensions = [".pl", ".PL", ".pm", ".t", ".psgi"]
root_markers = [".perl-lsp.toml", "Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"]
language_id = "perl"
```

### Bridge starts but returns empty results

- Start Codex from the project root.
- Confirm `--workspace` points at the repository root.
- Confirm the target file extension is listed in `extensions`.
- Confirm the project has a root marker such as `.perl-lsp.toml`, `Makefile.PL`,
  `Build.PL`, `cpanfile`, `dist.ini`, or `.git`.
- Try a file-specific diagnostic first:

  ```text
  Use the Perl LSP MCP tools to show diagnostics for path/to/file.pl.
  ```

### Rename does not apply changes automatically

Depending on the bridge, `rename` may preview a workspace edit rather than
write files directly. Ask Codex to inspect the preview and then apply the
returned edits.

### Slow first query

The bridge and `perllsp` may need to initialize and index the workspace. Retry
after the first request finishes. If the project is large, tune
`.perl-lsp.toml` or LSP initialization options rather than raising timeouts
indefinitely.

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP input from a
client. For manual checks, use:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
perllsp --check-project .
```

## Notes

- Keep one Codex session per project root for best workspace accuracy.
- Keep `lsp-mcp.toml` project-specific unless every project uses the same
  server mappings.
- Use project-local `.codex/config.toml` only in trusted repositories.
- Use `.perl-lsp.toml` for editor-agnostic `perl-lsp` settings.
