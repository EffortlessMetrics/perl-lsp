# OpenCode Setup Guide for perl-lsp

This guide shows how to run `perllsp` as a custom LSP server in OpenCode.

## Prerequisites

- `perllsp` installed and available on your `PATH`
- OpenCode installed
- a Perl project opened in OpenCode

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Configure OpenCode

Add a project-local `opencode.json` in your repository root (or update an existing
one) and register `perllsp` as a custom LSP.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "lsp": {
    "perl-lsp": {
      "command": ["perllsp", "--stdio"],
      "extensions": [".pl", ".pm", ".t", ".pod", ".psgi"],
      "initialization": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", "local/lib/perl5"]
          }
        }
      }
    }
  }
}
```

## Verify It Is Running

1. Open any Perl file (`.pl`, `.pm`, `.t`) in OpenCode.
2. Trigger a definition lookup or hover on a known symbol.
3. Confirm diagnostics appear for an intentional syntax error.

If OpenCode does not start the server, confirm `perllsp` is on your shell `PATH`
and restart OpenCode.

## Troubleshooting

- If no Perl files activate the server, verify your configured file extensions.
- If the command fails, run `perllsp --stdio` directly in a terminal to confirm
  the binary is launchable.
- For server-side behavior and config details, see
  [docs/reference/CONFIG.md](../reference/CONFIG.md) and
  [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
