# Claude Code Setup

This guide configures `perl-lsp` for Claude Code via an LSP plugin.

## Prerequisites

- `perllsp` is installed and on `PATH`.
- Claude Code is installed.
- You can create a local plugin directory.

Verify the language server first:

```bash
perllsp --version
perllsp --health
```

## Minimal local plugin

Create a local plugin with an `.lsp.json` file that starts `perllsp` over stdio.

```bash
mkdir -p .claude/plugins/perl-lsp
cat > .claude/plugins/perl-lsp/.lsp.json <<'JSON'
{
  "perl-lsp": {
    "command": "perllsp",
    "args": ["--stdio"],
    "extensionToLanguage": {
      ".pl": "perl",
      ".pm": "perl",
      ".t": "perl",
      ".psgi": "perl"
    }
  }
}
JSON
```

Start Claude Code with the plugin directory:

```bash
claude --plugin-dir .claude/plugins/perl-lsp
```

If Claude Code is already running, run `/reload-plugins` after edits.

## Pass Perl LSP settings through Claude Code

You can forward `initializationOptions` and `workspace/didChangeConfiguration`
settings directly to `perl-lsp`.

```json
{
  "perl-lsp": {
    "command": "perllsp",
    "args": ["--stdio"],
    "extensionToLanguage": {
      ".pl": "perl",
      ".pm": "perl",
      ".t": "perl",
      ".psgi": "perl"
    },
    "initializationOptions": {
      "perl": {
        "workspace": {
          "includePaths": ["lib", ".", "local/lib/perl5"],
          "useSystemInc": false
        }
      }
    },
    "settings": {
      "perl": {
        "inlayHints": {
          "enabled": true,
          "parameterHints": true
        }
      }
    }
  }
}
```

## Validation checklist

- Ask Claude Code to run `LSP goToDefinition` on a local symbol.
- Confirm diagnostics appear after introducing a syntax error.
- Confirm completions include workspace modules from your include paths.

## Troubleshooting

- If Claude cannot connect, confirm `perllsp --stdio` works from the same shell.
- If no files are recognized, verify `extensionToLanguage` includes your Perl extensions.
- If settings appear ignored, ensure JSON is valid and run `/reload-plugins`.
- If workspace modules are missing, set `perl.workspace.includePaths` in `initializationOptions`.

For all `perl-lsp` settings, see [`docs/reference/CONFIG.md`](../reference/CONFIG.md).
