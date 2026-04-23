# Amazon Kiro Setup Guide for perl-lsp

Amazon Kiro is built on Code OSS and can run VS Code-compatible extensions and
LSP settings. This guide gives the shortest path to getting `perllsp` running
reliably in Kiro.

## Prerequisites

- `perllsp` is installed and on your `PATH`
- Kiro can open your Perl workspace folder

Verify first:

```bash
perllsp --version
perllsp --health
```

## Option 1 (Recommended): Use the perl-lsp extension

If Kiro can install VS Code/Open VSX extensions in your environment, install the
`perl-lsp-rs` extension and keep defaults.

Then confirm:

1. Open any `.pl` or `.pm` file.
2. Trigger **Go to Definition** on a known symbol.
3. Confirm diagnostics appear for an intentional syntax error.

## Option 2: Configure a generic LSP server entry

If extension install is restricted, configure Kiro's LSP client so Perl files use
this command:

```text
perllsp --stdio
```

Use these initialization options (workspace settings or user settings):

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"],
      "useSystemInc": false
    },
    "inlayHints": {
      "enabled": true
    }
  }
}
```

## Troubleshooting

- If the language server fails to start, run `perllsp --health` in the same shell
  Kiro uses.
- If startup works but features are missing, open the Kiro LSP logs and confirm
  the process command is exactly `perllsp --stdio`.
- If module resolution is wrong, tune `perl.workspace.includePaths` first.

For broader diagnostics and recovery steps, see
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
