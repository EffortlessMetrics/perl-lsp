# Eclipse Setup Guide for perl-lsp

This guide shows how to use `perllsp` with Eclipse through the built-in LSP4E
language-server integration.

## Prerequisites

- Eclipse IDE with LSP4E support enabled (included in current Eclipse IDE
  packages)
- `perllsp` installed and available on your `PATH`
- A Perl project open in Eclipse

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Configure a Language Server Definition

1. Open **Preferences**.
2. Go to **Language Servers** (search for "language server" in Preferences).
3. Create a new server definition.
4. Set command to `perllsp --stdio`.
5. Associate the server with Perl file patterns (for example `*.pl`, `*.pm`,
   and `*.t`).
6. Apply the settings and restart Eclipse if prompted.

## Confirm the Server Is Running

1. Open a Perl file in the configured project.
2. Trigger **Go to Definition** on a symbol.
3. Trigger **Find References** and **Rename** to confirm requests are handled.
4. Check Eclipse logs if features do not activate:
   - **Window > Show View > Error Log**
   - Workspace `.metadata/.log`

## Workspace Tips

- Open the project root (not a single file) so workspace symbol indexing works.
- Keep `perllsp` on a stable path so Eclipse can restart it reliably.
- If diagnostics look stale, restart the language server from Language Servers
  preferences or restart Eclipse.

## Troubleshooting

If Eclipse launches but no language features appear:

1. Re-check command path with `which perllsp` (or `where perllsp` on Windows).
2. Confirm the Perl file extension mapping includes your files.
3. Confirm `perllsp --health` passes in the same environment Eclipse uses.
4. Use the project's [general troubleshooting guide](../how-to/TROUBLESHOOTING.md).
