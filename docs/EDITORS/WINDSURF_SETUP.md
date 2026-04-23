# Windsurf Setup Guide for perl-lsp

Windsurf is VS Code-compatible, so `perl-lsp` setup is nearly identical to VS Code.
Use this guide when you want a direct Windsurf configuration path.

## Prerequisites

- `perllsp` installed and available on your `PATH`
- Windsurf installed
- A Perl project folder opened as the workspace root

Verify your install before configuring the editor:

```bash
perllsp --version
perllsp --health
```

## Option 1: Use the perl-lsp Extension

If your Windsurf build can install the `perl-lsp-rs` extension, that is the
lowest-maintenance setup.

1. Open **Extensions** in Windsurf.
2. Search for `perl-lsp`.
3. Install the `perl-lsp-rs` extension when available.
4. Reload the window.

Then set the server path if needed:

```json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.trace.server": "off"
}
```

Leave `perl-lsp.serverPath` empty to use the extension's default resolution.
Set it to an absolute path if Windsurf cannot find `perllsp` on `PATH`.

## Option 2: Configure a Generic LSP Client

If extension marketplace access is restricted, configure a generic LSP client in
Windsurf and point it at `perllsp --stdio`.

Use this command tuple:

```json
{
  "command": "perllsp",
  "args": ["--stdio"]
}
```

Make sure the client is enabled for Perl file types (`.pl`, `.pm`, `.t`).

## Recommended Workspace Settings

Create or edit `.vscode/settings.json` in your project root:

```json
{
  "perl-lsp.includePaths": ["lib", "local/lib/perl5"],
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.trace.server": "off"
}
```

## Troubleshooting

- **Server not found**: set `perl-lsp.serverPath` to the absolute path for
  `perllsp`.
- **No diagnostics/completions**: verify the workspace root is the project root
  and Perl files are recognized by the LSP client.
- **Need deeper debugging**: set `perl-lsp.trace.server` to `"verbose"` and
  inspect Windsurf's LSP output panel.

For server-level issues, continue with
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
