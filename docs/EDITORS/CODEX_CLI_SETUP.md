# Codex CLI + perl-lsp Setup

This guide shows how to wire `perllsp` into Codex CLI so Codex can call real LSP
operations (`definition`, `hover`, `references`, `rename`, diagnostics) while it
is editing your Perl workspace.

## Prerequisites

- `perllsp` installed and on `PATH`
- Codex CLI installed and authenticated
- A project folder opened as your Codex working directory

Quick sanity checks:

```bash
perllsp --version
perllsp --health
codex --version
```

## 1) Add an LSP bridge MCP server

Codex CLI does not speak LSP directly; it calls tools via MCP. Configure an MCP
server that bridges MCP requests to your local language server.

Add/update `~/.codex/config.toml`:

```toml
[mcp_servers.perl_lsp]
command = "uvx"
args = ["lsp-mcp", "--workspace", "/absolute/path/to/project"]
```

If you do not use `uvx`, replace `command`/`args` with your preferred launcher
for the bridge server.

## 2) Register perl-lsp with the bridge

Most LSP bridge servers accept a per-language mapping. Add Perl so `.pl`, `.pm`,
`.t`, and `.pod` files resolve to `perllsp --stdio`.

Example `~/.config/lsp-mcp/config.toml`:

```toml
[language_servers.perl]
command = "perllsp"
args = ["--stdio"]
filetypes = ["perl"]
```

If your bridge uses JSON/YAML instead of TOML, keep the same values.

## 3) Validate from Codex

From inside your Perl repo:

```text
/mcp
```

Confirm the Perl LSP bridge is connected, then try prompts like:

- "Find all references to `My::Package::run`."
- "Rename `build_index` to `build_workspace_index` and update call sites."
- "Show diagnostics for this file and apply the safe fixes."

## Troubleshooting

- **No LSP tools appear in `/mcp`:** verify the bridge process starts outside
  Codex first, then restart Codex.
- **Server starts but returns empty results:** ensure Codex is running at the
  project root so workspace indexing can discover files.
- **`perllsp` not found:** run `which perllsp` and use an absolute path in the
  bridge config if needed.
- **Slow first queries:** this is usually initial indexing; retry after the
  first warmup pass.

## Notes

- Keep one Codex session per project root for best workspace accuracy.
- `perllsp --stdio` is the supported transport for editor and bridge clients.
