# Notepad++ Setup Guide for perl-lsp

This guide shows how to run `perllsp` from Notepad++ via the LSP client plugin.

## Prerequisites

- Notepad++ on Windows
- `perllsp` installed and available on `PATH`
- An LSP client plugin for Notepad++ (for example, **NppLspClient**)

Verify the server from a terminal first:

```powershell
perllsp --version
perllsp --health
```

## Configure the LSP client

In your Notepad++ LSP client settings, register a Perl server with:

- **command**: `perllsp`
- **arguments**: `--stdio`
- **language id**: `perl`
- **file extensions / associations**: `.pl`, `.pm`, `.t`, `.pod`, `.psgi`, `.cgi`

Example server command array:

```json
["perllsp", "--stdio"]
```

## Verify in editor

1. Restart Notepad++.
2. Open a Perl file (for example `script.pl`).
3. Confirm the LSP client reports the Perl server as started.
4. Try hover, completion, and diagnostics.

## Troubleshooting

- If the server cannot be started, use an absolute path to `perllsp.exe` in the plugin settings.
- If no features appear, verify the file is associated with Perl and the language id is `perl`.
- If startup works but features are inconsistent, run `perllsp --health` and then follow [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
