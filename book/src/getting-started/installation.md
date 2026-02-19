# Getting Started with perl-lsp

This guide gets you from zero to a working Perl language server in your editor.

## Prerequisites

- **Rust 1.89+** (for building from source)
- **A supported editor**: VS Code, Neovim, Emacs, Helix, or Sublime Text

## Installation

Choose one method:

### Option 1: Install from crates.io (Recommended)

```bash
cargo install perl-lsp
```

### Option 2: Download Pre-built Binary

Download from [GitHub Releases](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases):

```bash
# Linux (x86_64)
curl -LO https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases/latest/download/perl-lsp-linux-x86_64.tar.gz
tar xzf perl-lsp-linux-x86_64.tar.gz
sudo mv perl-lsp /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases/latest/download/perl-lsp-darwin-aarch64.tar.gz
tar xzf perl-lsp-darwin-aarch64.tar.gz
sudo mv perl-lsp /usr/local/bin/
```

### Option 3: Build from Source

```bash
git clone https://github.com/EffortlessMetrics/tree-sitter-perl-rs.git
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp
```

## Verify Installation

```bash
# Check binary is available
perl-lsp --version

# Quick health check
perl-lsp --health
# Should output: ok 0.9.0
```

## Quick Editor Setup

### VS Code

1. Install the extension:
   ```bash
   code --install-extension effortlesssteven.perl-lsp
   ```

2. Open a `.pl` or `.pm` file - the server starts automatically.

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
      root_dir = lspconfig.util.root_pattern('.git'),
      single_file_support = true,
    },
  }
end

lspconfig.perl_lsp.setup({})
```

### Emacs (with eglot, Emacs 29+)

```elisp
(add-to-list 'eglot-server-programs
             '((cperl-mode perl-mode) . ("perl-lsp" "--stdio")))
```

Then run `M-x eglot` in a Perl buffer.

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

## Your First 5 Minutes

Once installed, open any Perl file and try these features:

### 1. Hover for Documentation

Move your cursor over a function like `print` or `substr` and see documentation appear.

### 2. Go to Definition

Click on a variable or function call and use your editor's "Go to Definition" command:
- VS Code: `F12` or `Ctrl+Click`
- Neovim: `gd`
- Emacs: `M-.`

### 3. Find All References

Find everywhere a symbol is used:
- VS Code: `Shift+F12`
- Neovim: `gr`
- Emacs: `M-?`

### 4. Code Completion

Type `$` to see variable completions, or start typing a function name:

```perl
my $name = "Alice";
print $na  # Completes to $name
prin       # Completes to print
```

### 5. Quick Fixes

The LSP suggests fixes for common issues. Look for the lightbulb icon (VS Code) or use:
- VS Code: `Ctrl+.`
- Neovim: `<leader>ca`
- Emacs: `C-c l a`

## What You Get

perl-lsp provides:

| Feature | What It Does |
|---------|--------------|
| **Diagnostics** | Real-time syntax error detection |
| **Completion** | Variables, functions, keywords, file paths |
| **Hover** | Documentation for 150+ Perl built-ins |
| **Definition** | Jump to where symbols are defined |
| **References** | Find all uses of a symbol |
| **Rename** | Safely rename variables across files |
| **Formatting** | Format code with Perl::Tidy |
| **Folding** | Collapse functions, blocks, POD |
| **Symbols** | Document outline and workspace search |

## Project Configuration

For project-specific settings, the server reads configuration from your editor's LSP settings.

### Example: Configure Module Search Paths

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"]
    }
  }
}
```

### Example: Tune for Large Projects

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 50000,
      "referencesCap": 1000
    }
  }
}
```

See [CONFIG.md](CONFIG.md) for all configuration options.

## Troubleshooting

### Server Not Starting

```bash
# Test if the binary works
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
```

### No Diagnostics Appearing

1. Ensure your file has a Perl extension (`.pl`, `.pm`, `.t`)
2. Check your editor's language mode is set to Perl
3. Look at the LSP output log in your editor

### Slow Performance

Reduce indexed files and result caps in your settings:

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 5000,
      "workspaceSymbolCap": 100
    }
  }
}
```

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for more solutions.

## Next Steps

- **[EDITOR_SETUP.md](EDITOR_SETUP.md)** - Detailed editor configurations
- **[CONFIG.md](CONFIG.md)** - All configuration options
- **[LSP_FEATURES.md](LSP_FEATURES.md)** - Complete feature documentation
- **[FAQ.md](FAQ.md)** - Frequently asked questions

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues)
- **Documentation**: [docs/INDEX.md](INDEX.md)
