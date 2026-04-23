# Zed Setup Guide for perl-lsp

This guide gives you a minimal, reliable setup for using `perllsp` with Zed.

## Prerequisites

- Zed installed
- `perllsp` available on your `PATH`

Verify the binary first:

```bash
perllsp --version
perllsp --health
```

## 1) Configure the language server

Open Zed settings and add (or merge) this JSON:

```json
{
  "lsp": {
    "perllsp": {
      "command": {
        "path": "perllsp",
        "arguments": ["--stdio"]
      }
    }
  },
  "languages": {
    "Perl": {
      "language_servers": ["perllsp"]
    }
  }
}
```

If you keep project-specific Zed settings, use the same `lsp.perllsp` and
`languages.Perl.language_servers` fields there.

## 2) Verify attach

1. Restart Zed (or reload the window).
2. Open a `.pl` or `.pm` file.
3. Confirm language server activity in Zed's LSP logs/panel.

## 3) Optional server settings

You can forward `perllsp` configuration through initialization options:

```json
{
  "lsp": {
    "perllsp": {
      "command": {
        "path": "perllsp",
        "arguments": ["--stdio"]
      },
      "initialization_options": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"]
          }
        }
      }
    }
  }
}
```

For full setting names, see [Configuration Reference](../reference/CONFIG.md).

## Troubleshooting

- If Zed cannot start the server, run `perllsp --version` in a shell from the
  same environment as Zed and fix `PATH`.
- If the server starts but features are missing, confirm the file is detected
  as Perl and that `language_servers` includes `perllsp`.
- If startup fails after config edits, validate JSON formatting in settings.

For deeper diagnostics, see [Troubleshooting](../how-to/TROUBLESHOOTING.md).
