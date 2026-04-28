# Amazon Kiro Setup Guide for perl-lsp

This guide covers using `perl-lsp` with Amazon Kiro.

Kiro has two setup paths:

- **Kiro IDE**: use the OpenVSX extension `EffortlessMetrics.perl-lsp-rs`
- **Kiro CLI**: configure a custom workspace LSP that runs `perllsp --stdio`

## Prerequisites

### Kiro IDE

- Kiro IDE installed
- Perl workspace open in Kiro
- `EffortlessMetrics.perl-lsp-rs` installed from OpenVSX

The extension can auto-download `perllsp`. Manual `perllsp` installation is
mainly for offline, mirrored, or pinned-binary environments.

### Kiro CLI

- Kiro CLI installed
- `perllsp` installed and visible to the shell that launches Kiro CLI
- Project opened from repository root

Verify manual server installation when using CLI custom LSP:

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
4. Open `.pl`, `.pm`, or `.t` files and verify Perl language mode is active.

## Optional: Manual Binary Path (Kiro IDE)

Use this only when extension-managed download is blocked or you need a pinned
binary.

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

For protocol debugging only:

```json
{
  "perl-lsp.trace.server": "messages"
}
```

## Kiro CLI Setup

Run in your project root:

```text
/code init
```

Then edit the LSP config file created by Kiro.

Kiro docs currently describe this as project-root `lsp.json`. Some Kiro CLI
builds and examples reference `.kiro/settings/lsp.json`. Use the path your
installed Kiro CLI creates.

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

Refresh servers and inspect status/logs:

```text
/code init -f
/code status
/code logs
/code logs -l DEBUG -n 100
```

## Kiro CLI Caveat: Perl Is Custom

Kiro CLI's built-in language list does not currently include Perl. Treat custom
Perl LSP support as version-dependent and verify behavior in your installed
Kiro CLI build.

If diagnostics work but hover, references, definition, completion, or rename do
not, that may be a Kiro CLI custom-LSP limitation rather than a `perllsp`
server issue.

## Verification Checklist

### Kiro IDE

1. Open the project root.
2. Open a Perl file (`.pl`, `.pm`, `.t`).
3. Introduce a temporary syntax error.
4. Confirm diagnostics appear.
5. Remove the test syntax error.

### Kiro CLI

After `/code init`, ask for LSP-backed info and verify logs/status:

```text
Get diagnostics for lib/My/Module.pm
Find references of My::Module::some_function
What symbols are in lib/My/Module.pm?
What's the hover documentation for My::Module::some_function?
```

Also verify server behavior directly:

```bash
perllsp --check path/to/file.pl
```

## See Also

- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Configuration](../reference/CONFIG.md)
- [Troubleshooting](../how-to/TROUBLESHOOTING.md)
