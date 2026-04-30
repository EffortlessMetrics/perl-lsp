# Amazon Kiro Setup Guide for perl-lsp

This guide covers using `perl-lsp` with Amazon Kiro.

Kiro has two relevant setup paths:

- **Kiro IDE** — use the VS Code-compatible `EffortlessMetrics.perl-lsp-rs` extension from OpenVSX.
- **Kiro CLI** — use Kiro CLI workspace-scoped custom LSP configuration to launch `perllsp --stdio`.

## Prerequisites

### Kiro IDE

- Kiro IDE installed
- A Perl project opened in Kiro
- The `EffortlessMetrics.perl-lsp-rs` extension installed from OpenVSX

The extension can auto-download the matching `perllsp` binary. Install `perllsp`
manually only for offline environments, pinned internal deployments, or when
`perl-lsp.autoDownload` is disabled.

### Kiro CLI

- Kiro CLI installed
- `perllsp` installed and available to the shell that launches Kiro CLI
- A Perl project opened from the project root

Verify a manual `perllsp` installation:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Kiro IDE Setup

Kiro is built on Code OSS and uses OpenVSX-compatible extensions. Install:

```text
EffortlessMetrics.perl-lsp-rs
```

From Kiro:

1. Open the Extensions panel.
2. Search for `perl-lsp` or `EffortlessMetrics.perl-lsp-rs`.
3. Install the extension.
4. Open a Perl file such as `lib/My/Module.pm`, `script/app.pl`, or `t/basic.t`.

The extension should activate automatically for Perl files.

## Optional: Manual Binary Path for Kiro IDE

Use this only when extension-managed binary download is blocked or your team
pins a specific `perllsp` binary.

```json
{
  "perl-lsp.serverPath": "/absolute/path/to/perllsp",
  "perl-lsp.autoDownload": false
}
```

On macOS/Linux:

```bash
command -v perllsp
```

On Windows PowerShell:

```powershell
where perllsp
```

## Recommended Kiro IDE Settings

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

Use protocol tracing only while debugging:

```json
{
  "perl-lsp.trace.server": "messages"
}
```

Prefer `.perl-lsp.toml` for settings shared by the whole team:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]

[features]
inlay_hints = true
```

## Kiro CLI Setup

Kiro CLI has optional workspace-scoped LSP integration. Run this in the project
root:

```text
/code init
```

Then edit the LSP configuration file that Kiro creates.

Current Kiro docs describe this file as project-root `lsp.json`. Some Kiro CLI
examples and builds refer to `.kiro/settings/lsp.json`. Use the path created by
your installed Kiro CLI.

Add or merge this configuration:

```json
{
  "languages": {
    "perl": {
      "name": "perl-lsp",
      "command": "perllsp",
      "args": ["--stdio"],
      "file_extensions": [
        "pl",
        "PL",
        "pm",
        "t",
        "psgi",
        "cgi",
        "fcgi",
        "xs",
        "xsi"
      ],
      "project_patterns": [
        ".perl-lsp.toml",
        "Makefile.PL",
        "Build.PL",
        "cpanfile",
        "dist.ini",
        ".git"
      ],
      "exclude_patterns": [
        "**/.git/**",
        "**/local/**",
        "**/blib/**",
        "**/node_modules/**"
      ],
      "multi_workspace": false,
      "request_timeout_secs": 60,
      "initialization_options": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"],
            "useSystemInc": false,
            "resolutionTimeout": 50
          },
          "inlayHints": {
            "enabled": true,
            "parameterHints": true,
            "typeHints": true
          },
          "limits": {
            "workspaceSymbolCap": 200,
            "referencesCap": 500,
            "completionCap": 100
          }
        }
      }
    }
  }
}
```

Restart LSP servers after editing:

```text
/code init -f
```

Check status:

```text
/code status
```

View logs:

```text
/code logs
/code logs -l DEBUG -n 100
```

## Kiro CLI Caveat: Perl Is a Custom Language

Kiro CLI's built-in tree-sitter language list does not currently include Perl.
The custom LSP config above should still be useful for LSP-backed diagnostics
and semantic operations, but verify behavior in your installed Kiro CLI
version.

If diagnostics work but hover, references, go-to-definition, completion, or
rename do not, this may be a Kiro CLI custom-LSP limitation rather than a
`perllsp` problem.

## Verify It Is Running

### Kiro IDE

1. Open the project root in Kiro.
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

### Kiro CLI

Ask Kiro CLI for LSP-backed information after `/code init`:

```text
Get diagnostics for lib/My/Module.pm
Find references of My::Module::some_function
What symbols are in lib/My/Module.pm?
What's the hover documentation for My::Module::some_function?
```

Also check the server outside Kiro:

```bash
perllsp --check path/to/file.pl
```

## Troubleshooting

### Kiro IDE extension does not install

- Confirm the extension is available from OpenVSX.
- Update Kiro to a current build.
- If your organization uses a custom extension registry, confirm it mirrors `EffortlessMetrics.perl-lsp-rs`.
- If extension download is blocked by firewall or proxy policy, allow OpenVSX extension metadata and VSIX download hosts or install from an approved internal registry.

### Kiro IDE cannot find `perllsp`

If using extension auto-download, confirm:

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

If Kiro was launched from a GUI, it may not inherit your shell `PATH`. Use an
absolute `perl-lsp.serverPath` when needed.

### Kiro CLI cannot start `perllsp`

Check the binary from the same shell used to launch Kiro CLI:

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

On Windows PowerShell:

```powershell
where perllsp
perllsp --version
perllsp --health
perllsp --info
```

Then restart LSP servers:

```text
/code init -f
```

Check logs:

```text
/code logs -l ERROR
/code logs -l DEBUG -n 100
```

### No diagnostics or completion

- Confirm the file extension is listed in the Kiro CLI `file_extensions` array.
- Confirm the active document language is Perl in Kiro IDE.
- Confirm the workspace root is the project root, not a nested subdirectory.
- Check `perllsp` directly:

  ```bash
  perllsp --check path/to/file.pl
  ```

### Module resolution issues

Prefer `.perl-lsp.toml` for shared include paths:

```toml
[perl]
include_paths = ["lib", "local/lib/perl5", "vendor/lib"]
```

Or use editor / CLI initialization options:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
    }
  }
}
```

### `perllsp --stdio` appears to hang

That is expected. In stdio mode, `perllsp` waits for framed LSP JSON-RPC input
from the editor or CLI. Use these commands for manual checks instead:

```bash
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

## See Also

- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Configuration](../reference/CONFIG.md)
- [Troubleshooting](../how-to/TROUBLESHOOTING.md)
