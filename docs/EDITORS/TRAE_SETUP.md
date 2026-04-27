# Trae Setup Guide for perl-lsp

Trae can use VS Code-compatible extensions, so the recommended setup is the
same extension-based path used for VS Code.

## Prerequisites

- Trae installed
- A Perl project opened in Trae
- The `EffortlessMetrics.perl-lsp-rs` extension installed

The extension can auto-download the matching `perllsp` server binary on first
activation. Install `perllsp` manually only for offline environments, pinned
deployments, or when `perl-lsp.autoDownload` is disabled.

## Option 1: Install the perl-lsp Extension

Install the official Perl LSP extension:

```text
EffortlessMetrics.perl-lsp-rs
```

Try Trae's Extensions panel first. Search for `perl-lsp` or
`EffortlessMetrics.perl-lsp-rs`.

If it is not available in Trae's Extension Store, download the VSIX from the VS
Code Marketplace or Open VSX and install it through Trae's Extensions panel.

After installation, open a Perl file such as `lib/My/Module.pm`, `script/app.pl`,
or `t/basic.t`. The extension should activate for Perl files automatically.

## Optional: Manual `perllsp` Installation

Manual installation is useful when:

- extension-managed binary download is blocked by network/proxy policy
- your team pins a specific `perllsp` binary
- you use a generic LSP client extension instead of the official extension

Verify the binary:

```bash
perllsp --version
perllsp --health
perllsp --info
```

Then point the extension at it:

```json
{
  "perl-lsp.serverPath": "/absolute/path/to/perllsp",
  "perl-lsp.autoDownload": false
}
```

On macOS/Linux, find the path with:

```bash
command -v perllsp
```

On Windows PowerShell:

```powershell
where perllsp
```

## Option 2: Generic LSP Client Extension

Use this only if the official extension is unavailable.

Trae does not provide a documented bare settings-only way to register arbitrary
language servers. Install a generic LSP client extension first, then configure
that extension to launch:

```text
perllsp --stdio
```

For example, with a generic client that uses `lsp_generic_client` settings:

```json
{
  "lsp_generic_client": {
    "servers": {
      "perl-lsp": {
        "name": "Perl LSP",
        "path": "perllsp",
        "args": ["--stdio"],
        "documentSelector": ["perl"]
      }
    }
  }
}
```

The exact setting names depend on the generic LSP client extension. Use the
language ID `perl`; file extensions such as `.pl`, `.pm`, and `.t` are handled
by Trae's language/file association layer or by the installed Perl extension.

## Recommended Settings

For most users:

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5"]
}
```

Use trace logging only while debugging:

```json
{
  "perl-lsp.trace.server": "messages"
}
```

For team-shared project behavior, prefer `.perl-lsp.toml` at the repository
root:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]

[features]
inlay_hints = true
```

## Verify It Is Running

1. Open the project root as the Trae workspace.
2. Open a Perl file such as `.pl`, `.pm`, or `.t`.
3. Confirm the active document language is Perl.
4. Introduce a temporary syntax error.
5. Confirm diagnostics appear.
6. Remove the syntax error after testing.

Try standard editor actions:

- Go to Definition
- Find References
- Hover
- Rename Symbol
- Format Document

Available features depend on the installed extension, the active `perllsp`
binary, and the current file's language mode.

## Troubleshooting

### Extension does not install

- Update Trae to a current build.
- If Trae reports an engine or API compatibility error, try a newer Trae build
  or an earlier extension version.
- If Marketplace search does not show the extension, install from a downloaded
  `.vsix`.

### Server not found

If using the official extension, first check whether auto-download is enabled:

```json
{
  "perl-lsp.autoDownload": true
}
```

If using a manual binary:

```bash
perllsp --version
perllsp --health
perllsp --info
```

If Trae was launched from a GUI, it may not inherit the same `PATH` as your
terminal. Use an absolute `perl-lsp.serverPath` when needed.

### No diagnostics or completion

- Confirm the active document language is Perl.
- Confirm the workspace root is the project root, not a nested subdirectory.
- Check the Perl LSP output/log panel.
- Temporarily enable protocol tracing:

  ```json
  {
    "perl-lsp.trace.server": "messages"
  }
  ```

### Module resolution issues

Prefer `.perl-lsp.toml` for shared include paths:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]
```

Or use editor settings for a personal/workspace override:

```json
{
  "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
}
```

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input
from the editor. Use these commands for manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

## See Also

- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Configuration](../reference/CONFIG.md)
- [Troubleshooting](../how-to/TROUBLESHOOTING.md)
