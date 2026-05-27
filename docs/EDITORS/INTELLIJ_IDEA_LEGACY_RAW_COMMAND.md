# IntelliJ IDEA Legacy Raw Command Setup

Use this fallback only when the upstream LSP4IJ `perl-lsp` integration is not
available in your installed LSP4IJ build, when you are testing a local or
unreleased `perllsp` binary, or when you need temporary custom launch flags.

For normal JetBrains setup, start with
[IntelliJ IDEA / JetBrains Setup Guide](INTELLIJ_IDEA_SETUP.md) and use the
upstream LSP4IJ integration.

## Add a Raw Command Server

1. Open **File > Settings > Languages & Frameworks > Language Servers**.
2. Click **+** to add a new server definition.
3. Fill in the fields:

   | Field | Value |
   |-------|-------|
   | **Name** | `perl-lsp` |
   | **Command** | `perllsp --stdio` |
   | **Mappings: File name patterns** | `*.pl`, `*.pm`, `*.t`, `*.psgi`, `*.cgi` |

4. Click **OK** to save.

## Binary Path

If IntelliJ does not inherit your shell `PATH`, use an absolute binary path:

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

On Windows, use the full path with forward slashes or escaped backslashes:

```text
C:/path/to/perllsp.exe --stdio
```

## Descriptor Example

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

## Initialization Options

If you need server-specific startup settings, add an `initializationOptions` JSON
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

Prefer `.perl-lsp.toml` in the project root for settings that should apply
across all editors and teammates. Use LSP4IJ initialization options for
IDE-specific overrides.

## Verify

1. Open a Perl file matching one of the configured file patterns.
2. Confirm that the LSP4IJ status bar or LSP console shows `perllsp --stdio`
   starting successfully.
3. Introduce a temporary syntax error and confirm a diagnostic appears.
4. Remove the syntax error.

If the server does not start, verify the binary outside IntelliJ first:

```bash
perllsp --version
perllsp --health
```
