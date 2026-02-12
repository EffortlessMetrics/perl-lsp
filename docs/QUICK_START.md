# Quick Start Guide

Get up and running with perl-lsp in under 5 minutes.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Editor Setup](#editor-setup)
- [Verification](#verification)
- [Next Steps](#next-steps)

---

## Prerequisites

- **Rust** 1.89+ (for building from source)
- **A supported editor**: VS Code, Neovim, Emacs, Helix, or Sublime Text

---

## Installation

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

### Verify Installation

```bash
# Check version
perl-lsp --version

# Quick health check
perl-lsp --health
# Should output: ok 0.9.0
```

---

## Editor Setup

### VS Code

1. Install the extension:
   ```bash
   code --install-extension effortlesssteven.perl-lsp
   ```

2. Open a `.pl` or `.pm` file - the server starts automatically.

3. **Done!** You now have:
   - Syntax diagnostics
   - Go to definition (`F12`)
   - Find references (`Shift+F12`)
   - Code completion (`Ctrl+Space`)
   - Hover information (`Ctrl+I`)
   - Code actions (`Ctrl+.`)

**See:** [VS Code Setup Guide](EDITORS/VS_CODE_SETUP.md) for detailed configuration.

---

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

**Done!** You now have:
- Syntax diagnostics
- Go to definition (`gd`)
- Find references (`gr`)
- Code completion (`Ctrl+x Ctrl+o`)
- Hover information (`K`)

**See:** [Neovim Setup Guide](EDITORS/NEOVIM_SETUP.md) for detailed configuration.

---

### Emacs

**Option 1: Using eglot (Emacs 29+)**

```elisp
(use-package eglot
  :ensure t
  :hook ((cperl-mode . eglot-ensure)
         (perl-mode . eglot-ensure))
  :config
  (add-to-list 'eglot-server-programs
               '((cperl-mode perl-mode) . ("perl-lsp" "--stdio"))))
```

**Option 2: Using lsp-mode**

```elisp
(use-package lsp-mode
  :ensure t
  :hook ((cperl-mode . lsp-deferred)
         (perl-mode . lsp-deferred))
  :config
  (lsp-register-client
   (make-lsp-client
    :new-connection (lsp-stdio-connection '("perl-lsp" "--stdio"))
    :major-modes '(cperl-mode perl-mode)
    :server-id 'perl-lsp)))
```

**Done!** You now have:
- Syntax diagnostics
- Go to definition (`M-.`)
- Find references (`M-?`)
- Code completion (automatic)
- Hover information (hover)

**See:** [Emacs Setup Guide](EDITORS/EMACS_SETUP.md) for detailed configuration.

---

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "perl"
file-types = ["pl", "pm", "t", "psgi"]
roots = ["Makefile.PL", "Build.PL", "cpanfile", ".git"]
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]
```

**Done!** You now have:
- Syntax diagnostics
- Go to definition (`gd`)
- Find references (`gr`)
- Code completion (automatic)
- Hover information (`K`)

**See:** [Helix Setup Guide](EDITORS/HELIX_SETUP.md) for detailed configuration.

---

### Sublime Text

1. Install the [LSP](https://packagecontrol.io/packages/LSP) package via Package Control

2. Open `Preferences > Package Settings > LSP > Settings` and add:

```json
{
  "clients": {
    "perl-lsp": {
      "enabled": true,
      "command": ["perl-lsp", "--stdio"],
      "selector": "source.perl"
    }
  }
}
```

**Done!** You now have:
- Syntax diagnostics
- Go to definition (`F12`)
- Find references (`Shift+F12`)
- Code completion (`Ctrl+Space`)
- Hover information (`Ctrl+I`)

**See:** [Sublime Text Setup Guide](EDITORS/SUBLIME_SETUP.md) for detailed configuration.

---

### Vim/Neovim with coc.nvim

1. Install coc.nvim (see [coc.nvim Setup Guide](EDITORS/COC_NEOVIM_SETUP.md))

2. Create `~/.vim/coc-settings.json`:

```json
{
  "languageserver": {
    "perl": {
      "command": "perl-lsp",
      "args": ["--stdio"],
      "filetypes": ["perl"],
      "rootPatterns": ["Makefile.PL", "Build.PL", ".git"]
    }
  }
}
```

**Done!** You now have:
- Syntax diagnostics
- Go to definition (`gd`)
- Find references (`gr`)
- Code completion (`Ctrl+Space`)
- Hover information (`K`)

**See:** [coc.nvim Setup Guide](EDITORS/COC_NEOVIM_SETUP.md) for detailed configuration.

---

## Verification

### Test Basic Functionality

Create a test file `test.pl`:

```perl
#!/usr/bin/perl
use strict;
use warnings;

sub greet {
    my ($name) = @_;
    return "Hello, $name!";
}

my $message = greet("World");
print $message;
```

### Verify Features

1. **Syntax Diagnostics** - Try introducing an error:
   ```perl
   my $x = 1  # Missing semicolon
   ```
   Your editor should show an error.

2. **Go to Definition** - Place cursor on `greet` and navigate to definition.

3. **Find References** - Place cursor on `greet` and find all references.

4. **Code Completion** - Type `gre` and wait for completion suggestions.

5. **Hover Information** - Hover over `greet` to see documentation.

---

## Next Steps

### Learn More

- [Configuration Reference](CONFIGURATION_SCHEMA.md) - All configuration options
- [Performance Tuning](PERFORMANCE_TUNING.md) - Optimize for your setup
- [Troubleshooting Guide](TROUBLESHOOTING.md) - Common issues and solutions

### Editor-Specific Guides

- [VS Code Setup Guide](EDITORS/VS_CODE_SETUP.md)
- [Neovim Setup Guide](EDITORS/NEOVIM_SETUP.md)
- [Emacs Setup Guide](EDITORS/EMACS_SETUP.md)
- [Helix Setup Guide](EDITORS/HELIX_SETUP.md)
- [Sublime Text Setup Guide](EDITORS/SUBLIME_SETUP.md)
- [coc.nvim Setup Guide](EDITORS/COC_NEOVIM_SETUP.md)

### Advanced Features

- [DAP User Guide](DAP_USER_GUIDE.md) - Debugging with perl-dap
- [Workspace Navigation Guide](WORKSPACE_NAVIGATION_GUIDE.md) - Cross-file navigation
- [Import Optimizer Guide](IMPORT_OPTIMIZER_GUIDE.md) - Optimize imports

### Development

- [Getting Started](GETTING_STARTED.md) - Detailed getting started guide
- [LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md) - LSP server architecture
- [Development Guide](DEVELOPMENT.md) - Contributing to perl-lsp

---

## Common Issues

### Server Not Starting

**Problem:** perl-lsp doesn't start

**Solution:**
```bash
# Check if perl-lsp is in PATH
which perl-lsp

# Test server manually
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
```

### No Diagnostics

**Problem:** No errors shown for invalid code

**Solution:**
- Ensure file has `.pl`, `.pm`, or `.t` extension
- Check file type is set to "Perl" in your editor
- Restart your editor

### Slow Performance

**Problem:** Lag when typing

**Solution:**
- See [Performance Tuning Guide](PERFORMANCE_TUNING.md)
- Reduce result caps in configuration
- Disable system @INC if using network filesystems

### Module Resolution Issues

**Problem:** Can't find modules

**Solution:**
- Check `includePaths` in configuration
- Verify module exists: `perl -e 'use Module::Name;'`
- Ensure workspace root is correct

---

## Getting Help

- **Documentation:** [docs/](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/tree/main/docs)
- **Issues:** [GitHub Issues](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues)
- **Discussions:** [GitHub Discussions](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/discussions)

---

## License

perl-lsp is licensed under the Apache License 2.0. See [LICENSE](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/blob/main/LICENSE-APACHE) for details.

---

**Happy coding with Perl! 🐪**
