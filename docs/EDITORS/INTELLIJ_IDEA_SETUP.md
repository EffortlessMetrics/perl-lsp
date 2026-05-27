# IntelliJ IDEA / JetBrains Setup Guide for perl-lsp

This guide covers using `perl-lsp` with IntelliJ IDEA via the
[LSP4IJ](https://github.com/redhat-developer/lsp4ij) plugin.

LSP4IJ is a community plugin maintained by Red Hat that provides LSP client
support for JetBrains IDEs. This guide uses the upstream LSP4IJ `perl-lsp`
integration path.

Manual `perllsp --stdio` registration is still supported for older LSP4IJ
builds, local development, and custom launch flags. Keep that path separate:
use [Legacy Raw Command Setup](INTELLIJ_IDEA_LEGACY_RAW_COMMAND.md) only when
the upstream LSP4IJ entry is not available or you are testing an unreleased
server build.

> **Applies to:** IntelliJ IDEA (Community and Ultimate), as well as other
> JetBrains IDEs (PyCharm, WebStorm, Rider, etc.) that support LSP4IJ.

## Prerequisites

- IntelliJ IDEA 2024.2 or later (current LSP4IJ releases target the 2024.2+
  platform)
- The **LSP4IJ** plugin installed (see [Install LSP4IJ](#install-lsp4ij))
- `perllsp` installed and available on your `PATH`
- A Perl project opened in IntelliJ IDEA

Verify `perllsp` before configuring the IDE:

```bash
perllsp --version
perllsp --health
perllsp --info
```

If you still need to install the binary, see
[INSTALLATION.md](../how-to/INSTALLATION.md).

## Install LSP4IJ

1. Open **File > Settings** (IntelliJ IDEA > Settings on macOS) >
   **Plugins**.
2. Switch to the **Marketplace** tab.
3. Search for `LSP4IJ`.
4. Click **Install** and restart IntelliJ IDEA when prompted.

Alternatively, download the plugin from the
[JetBrains Marketplace](https://plugins.jetbrains.com/plugin/23257-lsp4ij)
and install it via **Plugins > Settings > Install Plugin from Disk**.

## Recommended: LSP4IJ Upstream Integration

Use this path when your installed LSP4IJ version includes a `perl-lsp` or Perl
language-server entry.

1. Install or update LSP4IJ.
2. Open a Perl file such as `lib/My/Module.pm` or `t/basic.t`.
3. Confirm LSP4IJ offers or enables the upstream `perl-lsp` server for Perl.
4. Set the `perllsp` binary path only if LSP4IJ asks for it or the binary is
   not visible on the IDE `PATH`.
5. Verify diagnostics, hover, go-to-definition, and inline completion if your
   LSP4IJ build exposes inline completion.

If LSP4IJ does not show a `perl-lsp` entry yet, update LSP4IJ first. If the
entry is still unavailable, use the
[legacy Raw Command fallback](INTELLIJ_IDEA_LEGACY_RAW_COMMAND.md).

## Binary Path

The upstream LSP4IJ entry should launch `perllsp` with stdio transport. If your
IDE does not inherit your shell `PATH`, set the binary path in the LSP4IJ
integration settings when prompted.

Find the binary path with `command -v perllsp` on Unix-like shells or
`where perllsp` in Windows PowerShell.

The upstream entry owns the server descriptor and file mappings. You should not
need to create a custom descriptor for normal setup.

## Initialization Options

To pass server-specific startup settings, add an `initializationOptions` JSON
block in the LSP4IJ **Server** tab under **Initialization options**:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"]
    },
    "inlayHints": {
      "enabled": true
    }
  }
}
```

Prefer `.perl-lsp.toml` in the project root for settings that should apply across
all editors and teammates. Use LSP4IJ initialization options for IDE-specific
overrides.

## File Type Activation

IntelliJ IDEA does not recognize Perl files by default unless the **Perl plugin**
is also installed. The upstream LSP4IJ `perl-lsp` integration provides the Perl
server mapping for common Perl extensions such as `*.pl`, `*.pm`, and `*.t`.

If you have the IntelliJ Perl plugin installed, IntelliJ already knows the `Perl`
file type. If Perl files still open as plain text, update LSP4IJ and confirm the
upstream `perl-lsp` integration is enabled. For older LSP4IJ builds, use the
[legacy Raw Command fallback](INTELLIJ_IDEA_LEGACY_RAW_COMMAND.md), which covers
manual file-pattern mapping.

## Optional: Inlay Hints

Recent LSP4IJ-supported IntelliJ builds surface LSP inlay hints. To enable them:

1. Open **File > Settings > Editor > Inlay Hints**.
2. Confirm that **LSP** hints are enabled.

Then enable inlay hints on the server side via initialization options (see above)
or in `.perl-lsp.toml`:

```toml
[features]
inlay_hints = true
```

## Optional: Inline Completion

`perl-lsp` supports the standard LSP 3.18 `textDocument/inlineCompletion`
request. Static clients receive the top-level `inlineCompletionProvider`
capability. LSP4IJ advertises dynamic inline-completion registration, so
`perllsp` registers `textDocument/inlineCompletion` with
`client/registerCapability` after `initialized` instead of also advertising a
duplicate static `inlineCompletionProvider`.

No LSP4IJ-specific server setting is required. If your LSP4IJ build exposes
inline completion, a simple Perl buffer containing `use ` should receive
deterministic suggestions such as `strict;`.

Protocol notes:

- Static clients receive top-level `inlineCompletionProvider: {}`.
- Dynamic-capable clients receive dynamic registration for
  `textDocument/inlineCompletion`.
- `experimental.inlineCompletionProvider` is not used.
- `experimental.perlInlineCompletionStream` is a custom extension for clients
  that explicitly integrate the streaming path.
- The registration selector includes `perl` and `perl5`.
- LSP wire positions use UTF-16 code units, per the LSP spec; `perllsp` converts client-provided positions to internal offsets before analysis.

## Verify Upstream LSP4IJ Behavior

1. Open a Perl file such as `lib/My/Module.pm` or `t/basic.t`.
2. Confirm that LSP4IJ activates: the status bar should show the language
   server indicator, and the LSP console should show `perllsp --stdio` starting
   successfully.
3. Introduce a temporary syntax error (e.g., remove a semicolon).
4. Confirm a diagnostic appears in the editor gutter.
5. Remove the syntax error.

Try LSP-backed navigation:

- **Go to Declaration**: `Ctrl+B` / `Cmd+B`
- **Find Usages**: `Alt+F7`
- **Hover**: mouse over a symbol or `Ctrl+Q` / `Ctrl+J`
- **Rename**: `Shift+F6`

Also confirm:

- workspace symbols do not retain closed virtual documents,
- inline completion is available if enabled by your LSP4IJ build,
- feature availability matches your LSP4IJ and IDE versions.

LSP4IJ exposes available LSP features through IntelliJ's standard action system.
Not all IntelliJ actions have LSP equivalents, but diagnostics, hover, go-to-definition,
references, and rename are supported.

## Troubleshooting

### Server does not start

Open **View > Tool Windows > LSP** (or the LSP Console from the status bar) to
see the LSP4IJ log. The console shows server startup commands and error output.

```bash
# Verify the binary outside IntelliJ first
perllsp --version
perllsp --health
```

If IntelliJ does not inherit your shell `PATH`, set the absolute `perllsp`
binary path in the upstream LSP4IJ integration settings. For older LSP4IJ builds,
use the [legacy Raw Command fallback](INTELLIJ_IDEA_LEGACY_RAW_COMMAND.md).

### `perllsp --stdio` appears to hang when run manually

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input.
It is not suitable for running interactively. Use these commands for manual checks:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

### No diagnostics for Perl files

- Confirm the upstream LSP4IJ `perl-lsp` integration is enabled.
- Confirm IntelliJ recognizes the file as Perl or that the file extension is a
  common Perl extension such as `*.pm`, `*.pl`, or `*.t`.
- Check that the LSP4IJ plugin is enabled in **Plugins**.
- Open the LSP Console to confirm the server started without error.

### Module resolution fails

Add the missing roots via initialization options:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
    }
  }
}
```

Or use `.perl-lsp.toml`:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]
```

### Restart the language server

From the LSP Console or status bar indicator, use the **Restart** action to
reload `perllsp` without restarting IntelliJ IDEA.

## Windows Notes

- Use the full path to `perllsp.exe` if it is not on the system `PATH`.
- IntelliJ on Windows may not inherit PowerShell `PATH` entries. Set the binary
  path explicitly in the LSP4IJ integration settings.
- Use forward slashes in paths or double backslashes:
  `C:/path/to/perllsp.exe`

## See Also

- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Legacy Raw Command Setup](INTELLIJ_IDEA_LEGACY_RAW_COMMAND.md)
- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [LSP4IJ Documentation](https://github.com/redhat-developer/lsp4ij/blob/main/docs/user-guide.md)
