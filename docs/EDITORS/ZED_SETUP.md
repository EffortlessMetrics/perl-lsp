# Zed Setup Guide for perl-lsp

Use this guide to wire `perllsp` into Zed via the built-in LSP client.

## Prerequisites

- `perllsp` installed and available on your `PATH`
- Zed updated to a recent stable release

Quick verification before changing editor settings:

```bash
perllsp --version
perllsp --health
```

## Basic Setup

Open your Zed settings (`~/.config/zed/settings.json` on Linux) and add:

```json
{
  "lsp": {
    "perl-lsp": {
      "command": {
        "path": "perllsp",
        "args": ["--stdio"]
      }
    }
  },
  "languages": {
    "Perl": {
      "language_servers": ["perl-lsp"]
    }
  }
}
```

If you already have a settings file, merge these keys into the existing JSON object.

## Optional Server Settings

You can pass the same `perl.*` LSP workspace settings used by other editors:

```json
{
  "lsp": {
    "perl-lsp": {
      "command": {
        "path": "perllsp",
        "args": ["--stdio"]
      },
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", "."]
          },
          "inlayHints": {
            "enabled": true
          }
        }
      }
    }
  }
}
```

## Troubleshooting

- If Perl files do not show diagnostics/completions, confirm Zed resolves `perllsp` from the same `PATH` as your shell.
- If the server fails to launch, run `perllsp --health` in a terminal first and fix installation issues before retrying.
- If behavior is still off, use [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
