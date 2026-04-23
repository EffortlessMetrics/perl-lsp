# Cursor Setup Guide for perl-lsp

Cursor is VS Code-compatible, so perl-lsp setup is almost identical: install
`perllsp`, then configure Cursor's language-server settings to launch
`perllsp --stdio`.

## Prerequisites

- `perllsp` on your `PATH`
- Cursor installed
- A Perl workspace folder open in Cursor

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Minimal Cursor Configuration

Create or update `.vscode/settings.json` in your project:

```json
{
  "perl.server": {
    "command": "perllsp",
    "args": ["--stdio"]
  }
}
```

If your Cursor setup already uses another LSP extension, configure that client
to run the same command and args.

## Recommended Project Settings

```json
{
  "perl.server": {
    "command": "perllsp",
    "args": ["--stdio"]
  },
  "perl": {
    "featureProfile": "auto",
    "includePaths": ["lib", "local/lib/perl5"]
  }
}
```

## Troubleshooting

- If the server does not start, run `perllsp --version` in the same shell used
  to launch Cursor and fix `PATH`.
- If diagnostics or completions are missing, restart the language server from
  the Command Palette and reopen the workspace folder.
- For general issues, see [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).

## Related Guides

- [VS Code setup](VS_CODE_SETUP.md) for extension-specific workflows
- [Configuration reference](../reference/CONFIGURATION.md) for server settings
