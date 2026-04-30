# OpenCode Setup Guide for perl-lsp

This guide shows how to run `perllsp` as a custom LSP server in OpenCode.

OpenCode uses LSP diagnostics to give the agent feedback about your code.
Direct hover, go-to-definition, references, and symbol operations are available
through OpenCode's experimental LSP tool.

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

If your project uses Perl-bearing template files, add their extensions as
needed, for example `.mason`, `.mas`, `.tt`, `.tt2`, or `.ep`.

Review project-local `opencode.json` before trusting it. Custom LSP commands run
local executables when matching files are opened.

## Optional: Pass perl-lsp Initialization Options

Prefer `.perl-lsp.toml` for settings that should apply across all editors. Use
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
            "includePaths": ["lib", ".", "local/lib/perl5"]
          }
        }
      }
    }
  }
}
```

## Verify It Is Running

1. Start OpenCode from the project root.
2. Open or reference a Perl file such as `lib/My/Module.pm`, `script/app.pl`, or
   `t/basic.t`.
3. Introduce a temporary syntax error.
4. Confirm diagnostics appear.
5. Remove the syntax error after verification.

You can also check a file outside OpenCode:

```bash
perllsp --check path/to/file.pl
```

## Optional: Enable Hover, Definition, and References

OpenCode's direct LSP tool is experimental. To let the agent call operations
such as hover, go-to-definition, references, document symbols, workspace
symbols, and call hierarchy, start OpenCode with:

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
- If OpenCode cannot start the server, run `command -v perllsp`,
  `perllsp --health`, and `perllsp --info` from the same shell environment used
  to launch OpenCode.
- On Windows PowerShell, use `where perllsp`.
- If `perllsp --stdio` appears to hang when run manually, that is expected. It
  is waiting for framed LSP input from the editor or agent.
- If module resolution fails, configure shared include paths in `.perl-lsp.toml`
  or pass `perl.workspace.includePaths` through OpenCode `initialization`.
- For OpenCode logs, start with debug logging:

  ```bash
  opencode --log-level DEBUG
  ```

- OpenCode logs are stored under `~/.local/share/opencode/log/` on macOS/Linux
  and `%USERPROFILE%\.local\share\opencode\log` on Windows.
- For server-side logging, temporarily add `--log` to the command or set
  `PERL_LSP_LOG=1` in the LSP `env` block.

Example server logging config:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "lsp": {
    "perl-lsp": {
      "command": ["perllsp", "--stdio", "--log"],
      "extensions": [".pl", ".PL", ".pm", ".t", ".pod", ".psgi"]
    }
  }
}
```

For server-side behavior and config details, see
[docs/reference/CONFIG.md](../reference/CONFIG.md) and
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
