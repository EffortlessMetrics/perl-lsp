# Zed Setup Guide for perl-lsp

This guide shows the fastest path to run `perllsp` in Zed.

## Prerequisites

- Zed installed (latest stable recommended)
- `perllsp` available on your `PATH`

Verify the binary before editing Zed settings:

```bash
perllsp --version
perllsp --health
```

## Configure the language server

Create or edit `~/.config/zed/settings.json` and add a Perl language server entry:

```json
{
  "lsp": {
    "perl-lsp": {
      "binary": {
        "path": "perllsp",
        "arguments": ["--stdio"]
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

If your binary is not on `PATH`, replace `"perllsp"` with an absolute path.

## Validate in Zed

1. Restart Zed.
2. Open a Perl project folder.
3. Open a `.pl` or `.pm` file and confirm:
   - diagnostics appear,
   - hover works,
   - completion suggestions appear while typing.

## Troubleshooting

- If Zed cannot launch the server, run `perllsp --health` in the same shell environment Zed inherits.
- If features are missing, confirm the file is detected as Perl and the project root is correct.
- See [`docs/how-to/TROUBLESHOOTING.md`](../how-to/TROUBLESHOOTING.md) for deeper debugging.
