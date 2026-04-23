# Roo Code Setup Guide for perl-lsp

Roo Code can run `perllsp` the same way as other stdio-based LSP clients.
This guide keeps the setup minimal and points to the VS Code guide for advanced
settings.

## Prerequisites

- `perllsp` is installed and available on `PATH`
- Roo Code has LSP support enabled for Perl files
- You have a workspace folder open (project root)

Verify the server binary first:

```bash
perllsp --version
perllsp --health
```

## Minimal LSP Command

Configure the Perl language server command as:

```text
perllsp --stdio
```

If your Roo Code build uses VS Code style settings JSON, use:

```json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true
}
```

## Recommended Workflow

1. Open a Perl file (`.pl`, `.pm`, or `.t`) inside your project workspace.
2. Confirm the language server process starts.
3. Run a quick smoke check:
   - go-to-definition on a known symbol
   - completion after typing `->`
   - diagnostics after introducing a temporary syntax error

## Troubleshooting

- If Roo Code cannot find the server, run `perllsp --version` in a terminal and
  fix `PATH`.
- If no diagnostics/completions appear, confirm the workspace root is the Perl
  project (not a parent directory with no source files).
- If startup works but behavior is incomplete, follow the deeper checks in
  [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).

## Advanced Configuration

For full initialization options (include paths, inlay hints, workspace limits,
feature toggles), use the same settings described in the VS Code guide:

- [docs/EDITORS/VS_CODE_SETUP.md](VS_CODE_SETUP.md)
