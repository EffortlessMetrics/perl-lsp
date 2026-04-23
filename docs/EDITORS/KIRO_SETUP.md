# Amazon Kiro Setup Guide for perl-lsp

This guide gives you a reliable setup path for using `perllsp` in Amazon Kiro.

## Prerequisites

- Amazon Kiro installed
- `perllsp` installed and available on `PATH`
- A Perl workspace opened in Kiro

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Kiro IDE (Desktop)

Kiro IDE is built on VS Code's open-source foundation and uses the OpenVSX
extension registry. Install the Perl LSP extension from the Extensions panel:

- Search for `perl-lsp` in Kiro's Extensions panel
- Extension ID: `EffortlessMetrics.perl-lsp-rs`

If the extension is not available in OpenVSX, download the `.vsix` from
[GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases) and
install it via "Install from VSIX..." in the Extensions panel menu.

## Kiro CLI

If you use the Kiro command-line interface, add a custom language server entry
in `.kiro/settings/lsp.json` at your project root:

```json
{
  "languages": {
    "perl": {
      "name": "perllsp",
      "command": "perllsp",
      "args": ["--stdio"],
      "file_extensions": ["pl", "pm", "t", "psgi"],
      "project_patterns": [".perl-lsp.toml", "Makefile.PL", "Build.PL"],
      "multi_workspace": false
    }
  }
}
```

Run `/code init` in the project root first if the `.kiro/settings/` directory
does not exist, then restart the Kiro CLI to load the new configuration.

## Recommended Workspace Settings

- Keep your project root open as the workspace root.
- Add include/library paths in project settings when your code uses nonstandard
  module locations.
- Restart the language server after changing server path or arguments.

## Troubleshooting

1. **Server does not start**
   - Run `perllsp --health` in an external shell.
   - Confirm Kiro inherits the same `PATH` as your shell.
2. **No diagnostics or completions**
   - Confirm the file is recognized as Perl.
   - Confirm the LSP client is attached to the current buffer/file.
3. **Definitions/references incomplete**
   - Open the repository root, not a nested subfolder.
   - Ensure local library paths are configured in project settings.

For cross-editor fallback patterns, see [Editor Setup](../how-to/EDITOR_SETUP.md)
and [Troubleshooting](../how-to/TROUBLESHOOTING.md).
