# Amazon Kiro Setup Guide for perl-lsp

This guide gives you a reliable setup path for using `perllsp` in Amazon Kiro.

## Prerequisites

- Amazon Kiro installed
- `perllsp` installed and available on `PATH`
- A Perl workspace opened in Kiro

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Quick Setup

Kiro is a desktop editor that can run stdio-based language servers. Configure Perl
files to launch:

```text
perllsp --stdio
```

If Kiro supports VS Code-compatible settings in your environment, the minimal
client shape is:

```json
{
  "command": "perllsp",
  "args": ["--stdio"],
  "filetypes": ["perl"]
}
```

## Recommended Workspace Settings

- Keep your project root open as the workspace root.
- Add include/library paths in project settings when your code uses nonstandard
  module locations.
- Restart the language server after changing server path or arguments.

## Troubleshooting

1. **Server does not start**
   - Run `perllsp --health` in an external shell.
   - Confirm Kiro inherits the same `PATH` as your shell.
2. **No diagnostics or completions**
   - Confirm the file is recognized as Perl.
   - Confirm the LSP client is attached to the current buffer/file.
3. **Definitions/references incomplete**
   - Open the repository root, not a nested subfolder.
   - Ensure local library paths are configured in project settings.

For cross-editor fallback patterns, see [Editor Setup](../how-to/EDITOR_SETUP.md)
and [Troubleshooting](../how-to/TROUBLESHOOTING.md).
