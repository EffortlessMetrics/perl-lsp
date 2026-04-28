# Amazon Kiro Setup Guide for perl-lsp

This guide covers using `perl-lsp` with Amazon Kiro.

Kiro has two setup paths:

- **Kiro IDE** — install the OpenVSX extension `EffortlessMetrics.perl-lsp-rs`
- **Kiro CLI** — configure a workspace custom language server that launches `perllsp --stdio`

## Prerequisites

### Kiro IDE

- Kiro IDE installed
- A Perl project opened in Kiro
- `EffortlessMetrics.perl-lsp-rs` installed from OpenVSX

The extension can auto-download `perllsp`. Manual binary installation is mainly
for offline, pinned, or policy-restricted environments.

### Kiro CLI

- Kiro CLI installed
- `perllsp` installed and available to the shell that launches Kiro CLI
- A Perl project opened from project root

Verify manual binary install:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Kiro IDE Setup

Kiro uses OpenVSX-compatible extensions. Install:

```text
EffortlessMetrics.perl-lsp-rs
```

From Kiro:

1. Open Extensions.
2. Search for `perl-lsp` or `EffortlessMetrics.perl-lsp-rs`.
3. Install the extension.
4. Open a `.pm`, `.pl`, or `.t` file.

## Optional: Manual Binary Path (Kiro IDE)

Use this only when extension-managed download is disabled or blocked:

```json
{
  "perl-lsp.serverPath": "/absolute/path/to/perllsp",
  "perl-lsp.autoDownload": false
}
```

## Recommended Kiro IDE Settings

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

Use protocol tracing only for debugging:

```json
{
  "perl-lsp.trace.server": "messages"
}
```

## Kiro CLI Setup

Kiro CLI supports workspace-scoped custom LSP entries.

Run in project root:

```text
/code init
```

Then edit the `lsp.json` file that Kiro creates.

Current Kiro docs describe this as project-root `lsp.json`; some Kiro CLI
examples/builds refer to `.kiro/settings/lsp.json`. Use the file path created
by your installed Kiro CLI.

Add or merge:

```json
{
  "languages": {
    "perl": {
      "name": "perl-lsp",
      "command": "perllsp",
      "args": ["--stdio"],
      "file_extensions": ["pl", "PL", "pm", "t", "psgi", "cgi", "fcgi", "xs", "xsi"],
      "project_patterns": [".perl-lsp.toml", "Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"],
      "exclude_patterns": ["**/.git/**", "**/local/**", "**/blib/**", "**/node_modules/**"],
      "multi_workspace": false,
      "request_timeout_secs": 60,
      "initialization_options": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5"],
            "useSystemInc": false,
            "resolutionTimeout": 50
          }
        }
      }
    }
  }
}
```

Restart servers:

```text
/code init -f
```

Check status and logs:

```text
/code status
/code logs
/code logs -l DEBUG -n 100
```

## Kiro CLI Caveat

Perl is not currently listed in Kiro CLI's built-in tree-sitter language set.
Treat custom Perl LSP support as version-dependent and verify locally.

If diagnostics work but hover/references/definition/completion/rename do not,
that can be a Kiro CLI custom-LSP limitation rather than a `perllsp` issue.

## Verify It Is Running

### Kiro IDE

1. Open the project root.
2. Open a Perl file.
3. Introduce a temporary syntax error.
4. Confirm diagnostics appear.
5. Remove the temporary error.

### Kiro CLI

After `/code init`, try LSP-backed queries and confirm responses are populated.
Also validate server behavior directly:

```bash
perllsp --check path/to/file.pl
```

## Troubleshooting

### Kiro IDE extension does not install

- Confirm the extension exists on OpenVSX.
- Update Kiro.
- If using an internal extension registry, confirm it mirrors `EffortlessMetrics.perl-lsp-rs`.

### Kiro IDE cannot find `perllsp`

If using auto-download, verify:

```json
{
  "perl-lsp.autoDownload": true
}
```

If using manual binary mode, run:

```bash
perllsp --version
perllsp --health
perllsp --info
```

Use absolute `perl-lsp.serverPath` if GUI launch does not inherit `PATH`.

### Kiro CLI cannot start `perllsp`

Run from the same shell used to launch Kiro CLI:

```bash
command -v perllsp
perllsp --version
perllsp --health
perllsp --info
```

Then restart and inspect logs:

```text
/code init -f
/code logs -l ERROR
/code logs -l DEBUG -n 100
```

## See Also

- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Configuration](../reference/CONFIG.md)
- [Troubleshooting](../how-to/TROUBLESHOOTING.md)
