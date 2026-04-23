# PearAI Setup Guide for perl-lsp

PearAI is VS Code-compatible, so `perl-lsp` setup is the same model: run
`perllsp` over stdio and configure settings in `settings.json`.

## Prerequisites

- `perllsp` is installed and available on your `PATH`
- PearAI can install VS Code marketplace-compatible extensions

Verify the binary first:

```bash
perllsp --version
perllsp --health
```

## Install the Extension in PearAI

Install the official extension:

- **Publisher**: `EffortlessMetrics`
- **Extension ID**: `EffortlessMetrics.perl-lsp-rs`

If PearAI cannot access the marketplace, configure a generic LSP client with:

```text
perllsp --stdio
```

## Recommended Settings

Add to your PearAI workspace settings JSON:

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.enableRefactoring": true
}
```

## Troubleshooting

- If the server is not detected, run `perllsp --version` in the same shell used
  to launch PearAI and fix `PATH`.
- If language features are missing, open the LSP/client logs in PearAI and
  confirm the server command is `perllsp --stdio`.
- For deeper diagnostics, follow [../how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).

## Related docs

- [VS Code setup](VS_CODE_SETUP.md)
- [General editor setup](../how-to/EDITOR_SETUP.md)
