# Zed Setup

This guide configures `perllsp` in [Zed](https://zed.dev/) using the built-in LSP
client.

## Prerequisites

- `perllsp` is installed and available on `PATH`
- Zed is updated to a recent stable release
- Your project root is opened as a Zed workspace

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Configure the language server in Zed

Open **Settings** in Zed and add/update your JSON config with a Perl language
entry and a `perllsp` language server.

```json
{
  "languages": {
    "Perl": {
      "language_servers": ["perllsp"]
    }
  },
  "lsp": {
    "perllsp": {
      "binary": {
        "path": "perllsp",
        "arguments": ["--stdio"]
      },
      "initialization_options": {
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
    }
  }
}
```

## Quick validation checklist

1. Open a `.pl` or `.pm` file.
2. Confirm diagnostics appear for obvious syntax errors.
3. Trigger completion on a Perl symbol and verify results appear.
4. Run "go to definition" on a local symbol.

If step 2–4 fail, open Zed's LSP logs and confirm `perllsp --stdio` started
without errors.

## Troubleshooting

- **`perllsp` not found**: start Zed from a shell where `perllsp --version`
  works, or use an absolute binary path in `lsp.perllsp.binary.path`.
- **No project symbols**: ensure you opened the project root folder, not only a
  single file.
- **Missing includes**: add paths under `initialization_options.perl.workspace.includePaths`.

For general LSP issues, continue with
[`docs/how-to/TROUBLESHOOTING.md`](../how-to/TROUBLESHOOTING.md).
