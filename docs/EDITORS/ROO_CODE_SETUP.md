# Roo Code Setup

Roo Code is VS Code-compatible, so `perl-lsp` setup is almost identical to
VS Code.

## Fast Path (Recommended)

1. Open **Extensions** in Roo Code.
2. Search for **Perl Language Server (perl-lsp-rs)** by `EffortlessMetrics`.
3. Install the extension.
4. Open a Perl workspace folder.

The extension downloads and manages the matching `perllsp` binary
automatically.

## Manual LSP Configuration

If you prefer not to install the extension, configure Roo Code's LSP client to
run:

```text
perllsp --stdio
```

Use your workspace root as the working directory so include paths and
configuration discovery resolve correctly.

## Settings You May Want

In `.vscode/settings.json` (Roo Code uses the same folder/settings model),
these are common project-level settings:

```json
{
  "perl-lsp.includePaths": [
    "lib",
    "local/lib/perl5"
  ],
  "perl-lsp.perlcritic.enabled": true,
  "perl-lsp.features.inlay_hints": true
}
```

## Troubleshooting

- Run `perllsp --health` in the same terminal environment Roo Code launches
  from.
- If modules do not resolve, verify `perl-lsp.includePaths` and check for a
  `.perl-lsp.toml` file at your workspace root.
- If the server does not start, open Roo Code's output/log panel and inspect
  language client startup errors.

For feature and behavior details, see the VS Code guide:
[VS_CODE_SETUP.md](./VS_CODE_SETUP.md).
