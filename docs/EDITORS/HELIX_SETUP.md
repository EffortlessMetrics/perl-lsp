# Helix Setup Guide for perl-lsp

Use this guide to run `perllsp` in Helix through Helix's built-in LSP client.

Helix already has built-in Perl language support. Its default Perl language
server is currently `perlnavigator`, so this guide shows how to override the
Perl language server to `perllsp`.

## Prerequisites

- Helix 23.10 or later; a current stable release is recommended
- `perllsp` installed and available on your `PATH`
- A Perl project opened from the project root

Verify both Helix and `perllsp` before editing configuration:

```bash
hx --health perl

perllsp --version
perllsp --health
perllsp --info
```

## Install `perllsp`

### Cargo

```bash
cargo install perllsp --locked
```

### From Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perllsp --locked
```

### Prebuilt Binary

Download the archive for your platform from GitHub Releases, extract it, and put
the `perllsp` binary on your `PATH`.

Check the release page before copying a version number. Release assets use the
`perllsp-<version>-<target>` naming pattern.

## Configure Helix

Create or update your Helix language configuration:

- Linux/macOS: `~/.config/helix/languages.toml`
- Windows: `%AppData%\helix\languages.toml`
- Project-local override: `.helix/languages.toml`

Add:

```toml
[language-server.perl-lsp]
command = "perllsp"
args = ["--stdio"]

[[language]]
name = "perl"
language-servers = ["perl-lsp"]
```

Restart Helix after changing `languages.toml`.

## Optional: Project-Specific Configuration

For one project, create `.helix/languages.toml` in the repository root:

```toml
[language-server.perl-lsp]
command = "perllsp"
args = ["--stdio"]

[[language]]
name = "perl"
language-servers = ["perl-lsp"]
```

Prefer `.perl-lsp.toml` for settings that should apply across all editors. Use
Helix `languages.toml` for Helix-specific wiring.

## Optional: Perl File Types

Helix file types are extensions without leading dots. If your project uses Perl
files beyond `.pl`, `.pm`, and `.t`, add them explicitly:

```toml
[[language]]
name = "perl"
file-types = ["pl", "PL", "pm", "t", "psgi", "cgi", "fcgi", "xs", "xsi"]
language-servers = ["perl-lsp"]
```

Avoid assigning `.pod` to Perl unless you intentionally want POD files handled as
Perl source; Helix has separate POD language support.

## Optional: Server Initialization Options

Helix sends `language-server.<name>.config` as LSP initialization options.

```toml
[language-server.perl-lsp.config.perl.workspace]
includePaths = ["lib", ".", "local/lib/perl5"]
useSystemInc = false
resolutionTimeout = 50

[language-server.perl-lsp.config.perl.inlayHints]
enabled = true
parameterHints = true
typeHints = true
maxLength = 30

[language-server.perl-lsp.config.perl.limits]
workspaceSymbolCap = 200
referencesCap = 500
completionCap = 100
```

For large workspaces, tune limits conservatively:

```toml
[language-server.perl-lsp.config.perl.limits]
workspaceSymbolCap = 100
referencesCap = 200
completionCap = 50
astCacheMaxEntries = 50
maxIndexedFiles = 5000
maxTotalSymbols = 250000
workspaceScanDeadlineMs = 20000
referenceSearchDeadlineMs = 1500
```

## Optional: Inlay Hints in Helix

`perllsp` can provide inlay hints, but Helix does not display them by default.
Enable them in `config.toml`:

```toml
[editor.lsp]
display-inlay-hints = true
```

You can also toggle them at runtime:

```text
:toggle lsp.display-inlay-hints
```

## Optional: Environment Variables

Use Helix's `environment` table instead of wrapping the command with `env`:

```toml
[language-server.perl-lsp]
command = "perllsp"
args = ["--stdio"]
environment = { PERL5LIB = "lib" }
```

## Verify It Is Running

1. Open a Perl file such as `lib/My/Module.pm`, `script/app.pl`, or `t/basic.t`.
2. Confirm the Helix statusline shows the language as `perl`.
3. Introduce a temporary syntax error.
4. Confirm diagnostics appear.
5. Remove the syntax error.

Useful checks:

```bash
hx --health perl
perllsp --health
perllsp --info
perllsp --check path/to/file.pl
```

Inside Helix:

```text
:log-open
:lsp-restart
:lsp-stop
:set-language perl
```

## Common Helix LSP Keybindings

| Action | Keybinding | Command |
| --- | --- | --- |
| Go to definition | `gd` | `goto_definition` |
| Find references | `gr` | `goto_reference` |
| Hover | `<space>k` | `hover` |
| Completion | `<C-x>` in insert mode | `completion` |
| Document symbols | `<space>s` | `symbol_picker` |
| Workspace symbols | `<space>S` | `workspace_symbol_picker` |
| Rename symbol | `<space>r` | `rename_symbol` |
| Code action | `<space>a` | `code_action` |
| Diagnostics picker | `<space>d` | `diagnostics_picker` |
| Workspace diagnostics | `<space>D` | `workspace_diagnostics_picker` |
| Next diagnostic | `]d` | `goto_next_diag` |
| Previous diagnostic | `[d` | `goto_prev_diag` |
| Format file | `:format` or `:fmt` | `format` |
| Format selection | `=` | `format_selections` |

## Formatting

Helix uses the language server for `:format` unless you configure an external
formatter. If formatting fails, confirm `perltidy` is installed:

```bash
perltidy --version
```

To use an external formatter instead of LSP formatting:

```toml
[[language]]
name = "perl"
formatter = { command = "perltidy", args = ["-q"] }
```

## Troubleshooting

### Server not starting

1. Confirm `perllsp` is launchable:

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

2. Confirm Helix sees the Perl language configuration:

   ```bash
   hx --health perl
   ```

3. Open the Helix log:

   ```text
   :log-open
   ```

4. Restart the server from Helix:

   ```text
   :lsp-restart
   ```

5. If `perllsp --stdio` appears to hang when run manually, that is expected. It
   is waiting for framed LSP input from the editor.

### No diagnostics

- Confirm the file language is `perl` in the statusline.
- Run `:set-language perl` if Helix detected the wrong language.
- Confirm `language-servers = ["perl-lsp"]` is attached to the `perl` language.
- Run `perllsp --check path/to/file.pl` outside Helix.

### Module resolution issues

Configure include paths in `.perl-lsp.toml` for shared project defaults, or pass
Helix-specific initialization options:

```toml
[language-server.perl-lsp.config.perl.workspace]
includePaths = ["lib", ".", "local/lib/perl5", "vendor/lib"]
useSystemInc = false
```

### Slow performance

Reduce result caps and indexing limits:

```toml
[language-server.perl-lsp.config.perl.limits]
workspaceSymbolCap = 100
referencesCap = 200
completionCap = 50
maxIndexedFiles = 5000
maxTotalSymbols = 250000
```

### Tree-sitter or highlighting issues

Run:

```bash
hx --health perl
```

If you build Helix from source or maintain a custom runtime, refresh grammars:

```bash
hx --grammar fetch
hx --grammar build
```

For server-side behaviour and configuration details, see
[docs/reference/CONFIG.md](../reference/CONFIG.md) and
[docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
