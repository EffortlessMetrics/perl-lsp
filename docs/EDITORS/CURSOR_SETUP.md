# Cursor Setup Guide for perl-lsp

This guide explains how to run `perllsp` in Cursor using the same extension
and settings model used by VS Code.

## Prerequisites

- Cursor installed
- `perllsp` installed and available on your `PATH`

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Install the Extension

1. Open **Extensions** in Cursor.
2. Search for `perl-lsp`.
3. Install **EffortlessMetrics.perl-lsp-rs**.

## Configure Cursor

Open workspace settings and add:

```json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off"
}
```

Notes:

- Leave `perl-lsp.serverPath` empty to use extension auto-download.
- Set `perl-lsp.serverPath` to an absolute binary path if your shell `PATH`
  differs from Cursor's app environment.

## Validate in Editor

Open a Perl file and confirm:

- diagnostics appear while typing
- hover information appears on built-ins
- go-to-definition works with `F12`

## Troubleshooting

- If Cursor cannot find `perllsp`, set `perl-lsp.serverPath` explicitly.
- If no LSP features appear, confirm the file language mode is Perl.
- If features are still missing, continue with
  [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
