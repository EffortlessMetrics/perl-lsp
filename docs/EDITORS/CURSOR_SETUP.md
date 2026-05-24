# Cursor Setup Guide for perl-lsp

This guide covers using `perl-lsp` with Cursor, the AI-assisted code editor.

Cursor is based on the VS Code codebase and can add language support through
extensions. Use the VS Code-compatible `perl-lsp` extension path:

- **Extension (recommended)** - install the `EffortlessMetrics.perl-lsp-rs` VSIX
  extension and let it manage `perllsp` automatically.

## Prerequisites

- A current Cursor build
- `perllsp` installed and on your `PATH` (optional when using the extension with
  auto-download enabled)
- A Perl project opened in Cursor

Verify `perllsp` before changing editor settings:

```bash
perllsp --version
perllsp --health
perllsp --info
```

If you still need to install the binary, see
[INSTALLATION.md](../how-to/INSTALLATION.md).

## Extension Setup

### Install the Extension

Cursor supports VS Code extension format. Install the `EffortlessMetrics.perl-lsp-rs`
extension using one of these methods:

**From the Extensions panel:**
1. Open the Extensions panel: `Ctrl+Shift+X` (Cmd+Shift+X on macOS).
2. Search for `perl-lsp` or `EffortlessMetrics.perl-lsp-rs`.
3. Click **Install**.

**From the command line:**
```bash
cursor --install-extension EffortlessMetrics.perl-lsp-rs
```

**From a downloaded VSIX:**
1. Download the `.vsix` from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases).
2. Open the Extensions panel, click the `...` menu, and choose **Install from VSIX**.

### Configure the Extension

The extension works without configuration using its defaults. For a workspace-specific
setup, create or edit `.vscode/settings.json` in your project root:

```json
{
  "perl-lsp.autoDownload": true,
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5"]
}
```

Cursor reads `.vscode/settings.json` for workspace-scoped settings, same as VS Code.
User-level settings are in Cursor's own settings file (accessible via
`Ctrl+,` / Cmd+,).

### Pin a Specific Release

To avoid tracking the `latest` channel:

```json
{
  "perl-lsp.channel": "tag",
  "perl-lsp.versionTag": "v0.15.0"
}
```

### Use a Pre-installed Binary

To disable auto-download and use a binary you installed:

```json
{
  "perl-lsp.serverPath": "/usr/local/bin/perllsp",
  "perl-lsp.autoDownload": false
}
```

## Optional: Server Initialization Options

Prefer `.perl-lsp.toml` for settings shared across all editors. Use
workspace settings for Cursor-specific startup settings that should not apply
everywhere.

```json
{
  "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
}
```

## Cursor AI and LSP

Cursor's AI features (tab completion, inline chat, composer) operate independently
of the LSP layer. Both run simultaneously without conflict. If Cursor's AI suggests
Perl completions and `perllsp` also fires, the two sources will appear in the same
completion list. This is expected behavior.

## Verify It Is Running

1. Open a Perl file such as `lib/My/Module.pm` or `t/basic.t`.
2. Confirm the language mode shows `Perl` in the status bar.
3. Introduce a temporary syntax error (e.g., remove a semicolon).
4. Confirm a diagnostic squiggle appears.
5. Remove the syntax error.

Try LSP-backed navigation:

- **Go to Definition**: `F12` or `Ctrl+Click` (Cmd+Click on macOS)
- **Find All References**: `Shift+F12`
- **Hover**: mouse over a symbol or `Ctrl+K Ctrl+I`
- **Rename Symbol**: `F2`
- **Format Document**: `Shift+Alt+F` (Shift+Option+F on macOS)

To restart the server without restarting Cursor:

1. Press `Ctrl+Shift+P` (Cmd+Shift+P on macOS).
2. Run **Perl: Restart Language Server**.

## Troubleshooting

### Server does not start

1. Verify the binary:
   ```bash
   perllsp --version
   perllsp --health
   ```

2. Open the Output panel (`Ctrl+Shift+U` / Cmd+Shift+U) and select
   **Perl Language Server** from the dropdown. Look for error messages.

3. Enable verbose tracing:
   ```json
   { "perl-lsp.trace.server": "verbose" }
   ```

4. Confirm Cursor was launched from a shell that has `perllsp` on its `PATH`.
   If Cursor was opened from a GUI launcher, it may not inherit shell `PATH`.
   Set an absolute `serverPath` in that case:
   ```json
   { "perl-lsp.serverPath": "/absolute/path/to/perllsp" }
   ```

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input.
It is not suitable for running manually without a client. Use these diagnostic
commands instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

### No diagnostics appear

- Confirm the file extension is `.pl`, `.pm`, or `.t`.
- Confirm the language mode in the status bar shows **Perl**.
- Confirm `perl-lsp.enableDiagnostics` is `true`.

### Module resolution fails

Add the missing roots:

```json
{ "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"] }
```

Or set them in `.perl-lsp.toml` (applies to all editors):

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]
```

### Duplicate diagnostics

If another Perl extension is installed (e.g., a different Perl LSP extension),
disable it. Open the Extensions panel, search for `perl`, and disable extensions
that register a language server.

## See Also

- [VS Code Setup](VS_CODE_SETUP.md) - the Cursor extension is the same package
- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [DAP User Guide](../tutorials/DAP_USER_GUIDE.md) - debugging support
