# Windsurf Setup Guide for perl-lsp

Windsurf is VS Code-compatible, so perl-lsp works with the same extension and
settings model.

## Prerequisites

- `perllsp` installed and on `PATH`
- Windsurf installed

Verify the binary first:

```bash
perllsp --version
perllsp --health
```

## Recommended: Install the official perl-lsp extension

Install `EffortlessMetrics.perl-lsp-rs` from the Windsurf extensions UI.

If you cannot access marketplace listings directly, install the packaged VSIX
from the project releases.

## Workspace settings

Create or update `.vscode/settings.json` in your project:

```json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.enableRefactoring": true
}
```

Windsurf reads this VS Code-compatible settings file and forwards those values
to the extension.

## Manual LSP fallback

If you use a generic LSP client instead of the extension, point it at:

```bash
perllsp --stdio
```

## Troubleshooting

- Confirm `perllsp --health` passes in a shell outside Windsurf.
- Open extension output/log panes and look for startup errors.
- Validate the workspace root is your project root so indexing can resolve
  local modules.
- For common recovery steps, see [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
