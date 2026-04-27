# OpenCode Setup Guide for perl-lsp

This guide shows how to run `perllsp` as a custom LSP server in OpenCode.

OpenCode is an AI coding agent rather than a traditional editor. Its LSP
integration is most useful for diagnostics by default. Hover, references, and
go-to-definition are available through OpenCode's experimental LSP tool.

## Prerequisites

- `perllsp` installed and available on your `PATH`
- OpenCode installed
- a Perl project opened from the project root

Verify the server first:

```bash
perllsp --version
perllsp --health
perllsp --info
```

## Configure OpenCode

Add a project-local `opencode.json` or `opencode.jsonc` in your repository root
(or update an existing one) and register `perllsp` as a custom LSP.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "lsp": {
    "perl-lsp": {
      "command": ["perllsp", "--stdio"],
      "extensions": [".pl", ".PL", ".pm", ".t", ".pod", ".psgi", ".cgi", ".fcgi", ".xs", ".xsi"]
    }
  }
}
```

If your project uses additional Perl-bearing template files, add their
extensions to the list, for example `.mason`, `.mas`, `.tt`, `.tt2`, or `.ep`.

## Optional: Pass perl-lsp Initialization Options

Prefer `.perl-lsp.toml` for settings that should apply to all editors. Use
OpenCode `initialization` only for OpenCode-specific startup options.

```json
{
  "$schema": "https://opencode.ai/config.json",
  "lsp": {
    "perl-lsp": {
      "command": ["perllsp", "--stdio"],
      "extensions": [".pl", ".PL", ".pm", ".t", ".pod", ".psgi", ".cgi", ".fcgi", ".xs", ".xsi"],
      "initialization": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
          }
        }
      }
    }
  }
}
```

## Verify It Is Running

1. Start OpenCode from the project root.
2. Open or reference a Perl file such as `lib/My/Module.pm` or `t/basic.t`.
3. Introduce a temporary syntax error and confirm OpenCode reports diagnostics.
4. Remove the syntax error after verification.

For OpenCode builds with the debug LSP command available, you can also test
diagnostics directly:

```bash
opencode debug lsp diagnostics path/to/file.pl
```

## Optional: Enable Hover, Definition, and References

OpenCode's direct LSP tool is experimental. To let the agent call operations
such as hover, go-to-definition, references, document symbols, and workspace
symbols, start OpenCode with:

```bash
OPENCODE_EXPERIMENTAL_LSP_TOOL=true opencode
```

Then allow the LSP tool in `opencode.json`:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "permission": {
    "lsp": "allow"
  }
}
```

You can combine this `permission` block with the `lsp` block above.

## Troubleshooting

- If no Perl files activate the server, verify the file extension is listed in
  `opencode.json`.
- If OpenCode cannot start the server, run `which perllsp` and `perllsp --health`
  from the same shell environment used to launch OpenCode.
- If `perllsp --stdio` appears to hang when run manually, that is expected: it is
  waiting for LSP JSON-RPC input.
- If module resolution fails, configure shared include paths in `.perl-lsp.toml`
  or pass `perl.workspace.includePaths` through OpenCode `initialization`.
- If OpenCode still does not report diagnostics, start it with debug logging and
  check the OpenCode logs.

For server-side behavior and config details, see
[docs/reference/CONFIG.md](../reference/CONFIG.md) and
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
