# Zed Setup Guide for perl-lsp

Use this guide to wire `perllsp` into Zed via the built-in LSP client.

## Prerequisites

- `perllsp` installed and available on your `PATH`
- Zed updated to a recent stable release (0.150+)

Quick verification before changing editor settings:

```bash
perllsp --version
perllsp --health
```

## How Zed Loads Language Servers

Zed discovers language servers through extensions. A Perl extension that
registers `perllsp` is the supported path for first-class Perl IDE features in
Zed (syntax highlighting, diagnostics, completions, go-to-definition, etc.).

If you already have a Zed Perl extension installed that launches `perllsp`, you
can override the binary path via `settings.json` using the `binary` key:

```json
{
  "lsp": {
    "perl-lsp": {
      "binary": {
        "path": "/usr/local/bin/perllsp",
        "arguments": ["--stdio"]
      }
    }
  }
}
```

The `binary` key tells Zed where to find the executable and which arguments to
pass instead of using whatever the extension resolves automatically. The key
name (`"perl-lsp"` above) must match the name your installed Perl extension
registers for the language server.

> **Note:** Without a Perl extension, Zed has no built-in mechanism to
> associate Perl files with `perllsp`. The `lsp` settings block only configures
> *known* language servers. For a fully generic LSP client approach, see
> [docs/how-to/EDITOR_SETUP.md](../how-to/EDITOR_SETUP.md).

## Optional Server Settings

Once Zed is launching `perllsp`, you can pass `perl.*` workspace configuration
through `initialization_options`:

```json
{
  "lsp": {
    "perl-lsp": {
      "initialization_options": {
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

- If Perl files do not show diagnostics/completions, confirm that a Zed Perl
  extension is installed and active (`Extensions` panel -> search "Perl").
- If the server fails to launch, run `perllsp --health` in a terminal first and
  fix installation issues before adjusting Zed settings.
- If behavior is still off, use [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
