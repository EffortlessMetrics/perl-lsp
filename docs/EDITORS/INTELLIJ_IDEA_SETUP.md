# IntelliJ IDEA Setup Guide for perl-lsp

This guide covers using `perl-lsp` with IntelliJ IDEA via the
[LSP4IJ](https://github.com/redhat-developer/lsp4ij) plugin.

LSP4IJ is a community plugin maintained by Red Hat that provides a generic LSP
client for all JetBrains IDEs. It lets you register any language server that speaks
stdio by pointing it at a shell command.

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

## Configure the perl-lsp Server

### Add a New Language Server

1. Open **File > Settings > Languages & Frameworks > Language Servers**.
2. Click **+** to add a new server definition.
3. Fill in the fields:

   | Field | Value |
   |-------|-------|
   | **Name** | `perl-lsp` |
   | **Command** | `perllsp --stdio` |
   | **Mappings: File name patterns** | `*.pl`, `*.pm`, `*.t`, `*.psgi`, `*.cgi` |

4. Click **OK** to save.

### Example Configuration

If your IntelliJ installation does not inherit your shell `PATH`, use an absolute
binary path:

| Field | Value |
|-------|-------|
| **Command** | `/usr/local/bin/perllsp --stdio` |

Find the path with:

```bash
command -v perllsp
```

On Windows PowerShell:

```powershell
where perllsp
```

Minimal descriptor/config example:

```json
{
  "name": "Perl Language Server",
  "languageId": "perl",
  "fileExtensions": ["pl", "pm", "t", "psgi"],
  "command": ["perllsp", "--stdio"]
}
```

The same example is checked in at
[`docs/EDITORS/lsp4ij-perl-lsp.json`](lsp4ij-perl-lsp.json).

On Windows, use the full path with forward slashes or escaped backslashes:

```
C:/path/to/perllsp.exe --stdio
```

### Initialization Options

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

## File Type Association

IntelliJ IDEA does not recognize Perl files by default unless the **Perl plugin**
is also installed. LSP4IJ maps language servers by file name pattern, so even
without the Perl plugin, LSP4IJ can activate `perllsp` for files matching
`*.pl`, `*.pm`, and `*.t`.

If you have the IntelliJ Perl plugin installed, IntelliJ already knows the `Perl`
file type. In that case, you can also map by file type instead of pattern in the
LSP4IJ settings.

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
request. LSP4IJ advertises dynamic inline-completion registration, so
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

## Verify It Is Running

1. Open a Perl file such as `lib/My/Module.pm` or `t/basic.t`.
2. Confirm that LSP4IJ activates: the status bar should show the language server
   indicator.
3. Introduce a temporary syntax error (e.g., remove a semicolon).
4. Confirm a diagnostic appears in the editor gutter.
5. Remove the syntax error.

Try LSP-backed navigation:

- **Go to Declaration**: `Ctrl+B` / `Cmd+B`
- **Find Usages**: `Alt+F7`
- **Hover**: mouse over a symbol or `Ctrl+Q` / `Ctrl+J`
- **Rename**: `Shift+F6`

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

If IntelliJ does not inherit your shell `PATH`, use an absolute path in the
**Command** field.

### `perllsp --stdio` appears to hang when run manually

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input.
It is not suitable for running interactively. Use these commands for manual checks:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

### No diagnostics for Perl files

- Confirm the file name pattern in LSP4IJ matches the file extension (e.g.,
  `*.pm`, `*.pl`, `*.t`).
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
  path explicitly in the LSP4IJ **Command** field.
- Use forward slashes in paths or double backslashes:
  `C:/path/to/perllsp.exe --stdio`

## See Also

- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Configuration Reference](../reference/CONFIG.md)
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md)
- [LSP4IJ Documentation](https://github.com/redhat-developer/lsp4ij/blob/main/docs/user-guide.md)
