# Getting Started with perl-lsp

Get from zero to a working Perl language server in under 2 minutes.

## Installation

Choose one method:

### Option 1: VS Code Extension (Easiest)

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

The extension auto-downloads the server binary. Open a `.pl` file and you are done.

### Option 2: Pre-built Binary

Download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases) and place the binary on your `PATH`.

### Option 3: From crates.io

```bash
cargo install perl-lsp
```

### Option 4: Build from Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perl-lsp
```

## Verify Installation

```bash
perl-lsp --health
# Output: ok 0.12.0
```

## Quick Editor Setup

### VS Code

Install the extension and open a Perl file -- everything works automatically:

```bash
code --install-extension effortlessmetrics.perl-lsp-rs
```

### Neovim

Add to your `init.lua`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perl-lsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern('.git', 'Makefile.PL', 'cpanfile', 'dist.ini'),
      single_file_support = true,
    },
  }
end

lspconfig.perl_lsp.setup({
  on_attach = function(client, bufnr)
    local opts = { buffer = bufnr, noremap = true, silent = true }
    vim.keymap.set('n', 'gd', vim.lsp.buf.definition, opts)
    vim.keymap.set('n', 'gr', vim.lsp.buf.references, opts)
    vim.keymap.set('n', 'K', vim.lsp.buf.hover, opts)
    vim.keymap.set('n', '<leader>rn', vim.lsp.buf.rename, opts)
    vim.keymap.set('n', '<leader>ca', vim.lsp.buf.code_action, opts)
    vim.keymap.set('n', '[d', vim.diagnostic.goto_prev, opts)
    vim.keymap.set('n', ']d', vim.diagnostic.goto_next, opts)
  end,
})
```

Verify: open a `.pl` file and run `:LspInfo`.

### Emacs (eglot, Emacs 29+)

```elisp
(add-to-list 'eglot-server-programs
             '((cperl-mode perl-mode) . ("perl-lsp" "--stdio")))
```

Run `M-x eglot` in a Perl buffer.

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "perl"
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]
```

### Other Editors

Any editor with LSP support works. Point it at `perl-lsp --stdio` as the language server command.

## What You Get

### Diagnostics

Parse errors appear as inline squiggly underlines the instant you open or edit a file. Hover over them for details.

### Code Completion

Start typing to see suggestions: variables in scope, 150+ built-in functions, module names, and your own subroutines.

```perl
my $name = "Alice";
print $na  # offers $name
prin       # offers print, printf, ...
use Fi     # offers File::Spec, File::Find, ...
```

### Hover Documentation

Hover over any built-in function (`print`, `substr`, `chomp`) or special variable (`$_`, `@ARGV`) for documentation, signatures, and examples.

### Go to Definition

Jump to where any symbol is defined -- variables go to `my`/`our`/`local`, subs go to `sub`, modules open the `.pm` file.

| Editor | Command |
|--------|---------|
| VS Code | `F12` or `Ctrl+Click` |
| Neovim | `gd` |
| Emacs | `M-.` |

### Find All References

Find every use of a symbol across your project.

| Editor | Command |
|--------|---------|
| VS Code | `Shift+F12` |
| Neovim | `gr` |
| Emacs | `M-?` |

### Rename Symbol

Rename a variable or subroutine across all files in one operation.

| Editor | Command |
|--------|---------|
| VS Code | `F2` |
| Neovim | `<leader>rn` |
| Emacs | `M-x eglot-rename` |

### Code Formatting

Format code with [Perl::Tidy](https://metacpan.org/pod/Perl::Tidy). Requires `perltidy` to be installed on your system:

```bash
cpanm Perl::Tidy
```

To configure, point to your `.perltidyrc` in VS Code settings:

```json
{ "perl-lsp.perltidyConfig": "/path/to/.perltidyrc" }
```

### Signature Help

When calling a function, parameter hints appear as you type.

### Code Actions

Lightbulb icon appears when fixable issues are detected. Trigger with `Ctrl+.` (VS Code), `<leader>ca` (Neovim), or `C-c l a` (Emacs).

### Semantic Highlighting

Enhanced syntax highlighting that understands context -- distinguishing variables, functions, packages, and special variables beyond what regex-based highlighting can do.

### Code Lens

Reference counts appear above subroutine definitions, showing how many places call each function.

### Additional Features

| Feature | Description |
|---------|-------------|
| Document symbols | Outline of subs, packages, and variables |
| Workspace symbols | Search any symbol across your project (`Ctrl+T`) |
| Inlay hints | Parameter names and type hints inline |
| Call hierarchy | Navigate caller/callee chains |
| Type hierarchy | Navigate class/role inheritance |
| Folding | Collapse functions, blocks, POD sections |
| Selection range | Smart expand/shrink selection |
| Linked editing | Edit matching tokens simultaneously |
| Document links | Clickable links to modules and documentation |
| Color decorators | Color preview for color literals |

## Configuration

### VS Code Settings

The extension exposes these settings (search "perl-lsp" in VS Code settings):

| Setting | Default | Description |
|---------|---------|-------------|
| `perl-lsp.serverPath` | _(auto)_ | Absolute path to `perl-lsp` binary |
| `perl-lsp.autoDownload` | `true` | Auto-download binary if not found |
| `perl-lsp.enableDiagnostics` | `true` | Real-time syntax diagnostics |
| `perl-lsp.enableSemanticTokens` | `true` | Semantic syntax highlighting |
| `perl-lsp.enableFormatting` | `true` | Document formatting (needs `perltidy`) |
| `perl-lsp.formatOnSave` | `false` | Format on save |
| `perl-lsp.enableRefactoring` | `true` | Refactoring code actions |
| `perl-lsp.perltidyConfig` | _(auto)_ | Path to `.perltidyrc` |
| `perl-lsp.includePaths` | `["lib", "local/lib/perl5"]` | Module search paths |
| `perl-lsp.featureProfile` | `auto` | Feature profile (see below) |
| `perl-lsp.trace.server` | `off` | LSP trace level (`off`, `messages`, `verbose`) |

### Feature Profiles

Feature profiles control which LSP capabilities are advertised:

| Profile | Description |
|---------|-------------|
| `auto` | Follow binary build mode (default) |
| `ga-lock` | Only GA-locked stable features |
| `ga` | All GA features |
| `prod` / `production` | Production-ready feature set |
| `all` | Every implemented feature |

Set via CLI (`--feature-profile prod`) or VS Code setting (`perl-lsp.featureProfile`).

### Module Search Paths

Configure where perl-lsp looks for modules:

```json
{
  "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
}
```

### Large Projects

Tune resource limits for large codebases:

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 50000,
      "referencesCap": 1000,
      "workspaceSymbolCap": 100
    }
  }
}
```

## CLI Reference

```
perl-lsp [options]
perl-lsp --check <file.pl> [file2.pm ...]
```

| Flag | Description |
|------|-------------|
| `--stdio` | Use stdio for communication (default) |
| `--socket` | Use TCP socket for communication |
| `--port <N>` | Port for socket mode (default: 9257) |
| `--log` | Enable logging to stderr |
| `--health` | Quick health check: prints `ok <version>` |
| `--info` | Show version, feature profile, LSP coverage |
| `--check <files>` | Validate Perl files and report parse errors |
| `--version` | Print version and git tag |
| `--features-json` | Output feature catalog as JSON |
| `--feature-profile <name>` | Set feature profile (`auto`, `ga-lock`, `ga`, `prod`, `all`) |
| `--completion <shell>` | Generate shell completions (`bash`, `zsh`, `fish`, `powershell`) |
| `--help` | Show help |

### Examples

```bash
# Run as LSP server (default, what editors use)
perl-lsp --stdio

# Quick validation of Perl files
perl-lsp --check lib/MyModule.pm script.pl

# Show server capabilities
perl-lsp --info

# Run with logging for debugging
perl-lsp --stdio --log

# Run in socket mode
perl-lsp --socket --port 9257

# Install shell completions
perl-lsp --completion bash >> ~/.bashrc
perl-lsp --completion zsh >> ~/.zshrc
perl-lsp --completion fish > ~/.config/fish/completions/perl-lsp.fish
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `PERL_LSP_LOG` | Set log filter (e.g., `perl_lsp=debug`) |
| `RUST_LOG` | Alternative log filter |
| `NO_COLOR` | Disable colored output |

## Troubleshooting

### "Binary not found" after install

`cargo install` places the binary in `~/.cargo/bin/`. If your shell cannot find `perl-lsp`:

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"
source ~/.bashrc
```

### Editor not connecting

1. Verify the binary: `which perl-lsp && perl-lsp --health`
2. Launch your editor from the terminal so it inherits your `PATH`
3. Check editor logs:
   - VS Code: View > Output > "Perl Language Server"
   - Neovim: `:LspLog`
   - Emacs: `*eglot stderr*` buffer

### Formatting not working

Install perltidy: `cpanm Perl::Tidy`

### No diagnostics

1. Check file extension is `.pl`, `.pm`, or `.t`
2. Verify editor language mode is set to Perl
3. Check `perl-lsp.enableDiagnostics` is `true`

### Debugging server issues

```bash
# Test JSON-RPC directly
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' \
  | perl-lsp --stdio

# Run with verbose logging
RUST_LOG=perl_lsp=debug perl-lsp --stdio 2>debug.log
```

## Next Steps

- **[DAP User Guide](DAP_USER_GUIDE.md)** -- Set up the built-in debugger (breakpoints, stepping, watch expressions)
- **[Editor Setup](../how-to/EDITOR_SETUP.md)** -- Detailed per-editor configurations
- **[Configuration Reference](../reference/CONFIG.md)** -- All configuration options
- **[LSP Features](../reference/LSP_FEATURES.md)** -- Complete feature documentation
- **[FAQ](../reference/FAQ.md)** -- Frequently asked questions
- **[Troubleshooting](../how-to/TROUBLESHOOTING.md)** -- Full troubleshooting guide

## Getting Help

- **Issues**: [github.com/EffortlessMetrics/perl-lsp/issues](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Documentation**: [docs/](../INDEX.md)
