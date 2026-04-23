# Windsurf Setup Guide for perl-lsp

This guide shows the fastest way to use `perllsp` in Windsurf.

## Prerequisites

- Windsurf installed
- `perllsp` available on your `PATH`

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Option 1: Install the perl-lsp extension from Open VSX

Windsurf uses Open VSX for extensions. Install `perl-lsp-rs` from publisher
`EffortlessMetrics` in the Extensions panel.

After installing, open a Perl file (`.pl`, `.pm`, `.t`) and confirm the status
bar shows the Perl language server is running.

## Option 2: Configure perllsp manually with a generic LSP extension

If you prefer a generic LSP client in Windsurf, configure it to launch:

```text
perllsp --stdio
```

Use your project root as the workspace folder so module resolution and
workspace-wide features behave correctly.

## Recommended settings

In Windsurf settings JSON, keep these defaults unless you have a reason to tune
behavior:

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.enableRefactoring": true
}
```

If `perllsp` is already installed system-wide, set `perl-lsp.serverPath` to the
absolute path of your binary.

## Troubleshooting

- If startup fails, run `perllsp --health` in the same shell environment used by
  Windsurf.
- If features are missing, make sure the workspace root is the project root and
  not a nested subdirectory.
- For deeper diagnostics, temporarily set `"perl-lsp.trace.server": "verbose"`
  and inspect Windsurf's extension/LSP logs.

See also: [Editor Setup](../how-to/EDITOR_SETUP.md),
[TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
