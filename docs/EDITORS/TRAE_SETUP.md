# Trae (ByteDance) Setup Guide for perl-lsp

Trae is compatible with the VS Code extension ecosystem, so perl-lsp setup is
nearly identical to VS Code.

## Prerequisites

- `perllsp` available on your `PATH`
- Trae installed

Verify the server before configuring the editor:

```bash
perllsp --version
perllsp --health
```

## Option 1: Install the perl-lsp extension (recommended)

If Trae can access Open VSX, install:

- `EffortlessMetrics.perl-lsp-rs`

Then open Trae settings and confirm the extension is enabled for Perl files.

## Option 2: Generic LSP client configuration

If you prefer manual wiring, configure a generic language server entry:

- **Command**: `perllsp`
- **Arguments**: `--stdio`
- **Filetypes / language IDs**: `perl`, `pl`, `pm`, `t`

## Recommended settings

- Open the project root as your workspace folder (not a nested subdirectory)
- Keep `perl-lsp.trace.server` at `messages` while debugging setup
- Put team-shared behavior in `.perl-lsp.toml` at repo root

## Troubleshooting

1. If Trae reports "server not found", run `perllsp --version` in a shell
   started the same way you launch Trae.
2. If diagnostics/completion do not appear, verify the active document language
   is Perl and check the LSP output/log panel.
3. If modules are unresolved, add include paths in `.perl-lsp.toml` (`include_paths`)
   or editor settings (`perl.includePaths`).

See also:

- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Configuration](../reference/CONFIGURATION.md)
- [Troubleshooting](../how-to/TROUBLESHOOTING.md)
