# Getting Started with perllsp

This guide gets you from zero to a working Perl language server in your editor.

## First Success

The fastest validation path is:

1. Install `perllsp`
2. Run `perllsp --health`
3. Open a Perl file in your editor
4. Confirm you get diagnostics and hover text

If those four steps work, your install is good and the rest of this guide is
just editor-specific setup and feature discovery.

## What is a Language Server?

A **language server** is a program that runs alongside your editor and gives it deep understanding of your code. Instead of each editor re-implementing features like "go to definition" or "show all references," the [Language Server Protocol (LSP)](https://microsoft.github.io/language-server-protocol/) defines a standard way for any editor to talk to a language-specific backend. `perllsp` is the native Perl 5 language server CLI from the perl-lsp project: it parses your code, builds an index of symbols, and responds to editor requests over JSON-RPC -- so you get IDE-grade navigation, completion, diagnostics, and refactoring in VS Code, Neovim, Emacs, Helix, or any other LSP-capable editor. No Perl runtime is required; the server is a single native binary.

## Prerequisites

- **Rust 1.92+** (for building from source)
- **A supported editor**: VS Code, Neovim, Emacs, Helix, or Sublime Text

## Installation

Choose one method:

### Option 1: VS Code extension (Recommended for VS Code users)

```bash
code --install-extension EffortlessMetrics.perl-lsp-rs
```

The extension downloads the matching server binary for your platform.

### Option 2: GitHub release binary (Recommended for other editors)

Download the latest archive from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases) and place `perllsp` on your `PATH`.

### Option 3: Install Script (Linux/macOS)

Use the installer script (best-effort / non-canonical):

```bash
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash
```

### Option 4: Build from Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd perl-lsp
cargo install --path crates/perllsp
```

## Verify Installation

```bash
# Check binary is available
perllsp --version

# Quick health check
perllsp --health
# Should output: ok <installed-version>

# Optional: show feature/profile information
perllsp --info

# Optional: validate a Perl file from the CLI
perllsp --check script.pl
```

If `--version` and `--health` work but your editor still cannot connect, jump to [Troubleshooting](../how-to/TROUBLESHOOTING.md).

## Quick Editor Setup

### VS Code

1. Install the extension:
   ```bash
   code --install-extension EffortlessMetrics.perl-lsp-rs
   ```

2. Open a `.pl` or `.pm` file - the server starts automatically.

### Neovim

Add to your `init.lua`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

-- Register the Perl LSP server with nvim-lspconfig
if not configs.perl_lsp then
  configs.perl_lsp = {
    default_config = {
      cmd = { 'perllsp', '--stdio' },
      filetypes = { 'perl' },
      root_dir = lspconfig.util.root_pattern('.git', 'Makefile.PL', 'cpanfile', 'dist.ini'),
      single_file_support = true,
      settings = {
        perl = {
          workspace = {
            includePaths = { 'lib', '.', 'local/lib/perl5' },
          },
        },
      },
    },
  }
end

lspconfig.perl_lsp.setup({
  on_attach = function(client, bufnr)
    -- Suggested keybindings (customize to taste)
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

**Verify it works**: open a `.pl` file and run `:LspInfo` -- you should see `perl_lsp` attached.

### Emacs (with eglot, Emacs 29+)

```elisp
(add-to-list 'eglot-server-programs
             '((perl-mode cperl-mode perl-ts-mode) . ("perllsp" "--stdio")))
```

Then run `M-x eglot` in a Perl buffer.

### Helix

Add to `~/.config/helix/languages.toml`:

```toml
[[language]]
name = "perl"
language-servers = ["perllsp"]

[language-server.perllsp]
command = "perllsp"
args = ["--stdio"]
```

## Your First 5 Minutes

Once installed, open any Perl file and try these features. Each heading describes what you will see in your editor.

### 1. Real-Time Diagnostics

As soon as you open a Perl file, the server parses it and reports errors. You will see **red or yellow squiggly underlines** directly on lines with problems, just like a spell-checker. A count badge appears in your editor's status bar or problems panel. Hover over a squiggle to read the error message inline.

### 2. Hover for Documentation

Move your cursor over a built-in function like `print`, `substr`, or `chomp`. After a brief pause, a **floating tooltip** appears with the function signature, a short description, and a usage example. This works for over 150 Perl built-ins, keywords, and special variables like `$_` and `@ARGV`.

### 3. Code Completion

Start typing and the server offers completions in a **dropdown list** that appears automatically. Type `$` to see variable names in scope, `use ` to see module names, or the first few letters of a function to see matching built-ins and your own subroutines. The list filters as you type.

```perl
my $name = "Alice";
print $na  # Dropdown offers $name
prin       # Dropdown offers print, printf, ...
use Fi     # Dropdown offers File::Spec, File::Find, ...
```

### 4. Go to Definition

Place your cursor on a variable, function call, or module name and jump to where it is defined.

| Editor | Command |
|--------|---------|
| VS Code | `F12` or `Ctrl+Click` |
| Neovim | `gd` |
| Emacs | `M-.` |

The editor opens the target file and scrolls to the exact line. For variables, it jumps to the `my`, `our`, or `local` declaration. For subroutines, it jumps to the `sub` definition. For modules, it opens the `.pm` file.

### 5. Find All References

Find every place a symbol is used across your project. Results appear in a **references panel** (VS Code) or a quickfix list (Neovim).

| Editor | Command |
|--------|---------|
| VS Code | `Shift+F12` |
| Neovim | `gr` |
| Emacs | `M-?` |

### 6. Rename Symbol

Rename a variable or subroutine and the server updates **every reference** across files in a single operation. Your editor shows a preview of all changes before applying them.

| Editor | Command |
|--------|---------|
| VS Code | `F2` |
| Neovim | `<leader>rn` |
| Emacs | `M-x eglot-rename` |

### 7. Document Outline and Symbols

Open your editor's symbol outline to see a **tree of subroutines, packages, and variables** in the current file. Use workspace symbol search (`Ctrl+T` in VS Code, `<leader>ws` in Neovim) to jump to any symbol across your project.

### 8. Code Actions and Quick Fixes

When the server detects a fixable issue, a **lightbulb icon** appears in the gutter (VS Code) or a hint appears in the diagnostic. Trigger the action to apply the fix automatically.

| Editor | Command |
|--------|---------|
| VS Code | `Ctrl+.` |
| Neovim | `<leader>ca` |
| Emacs | `C-c l a` |

## What You Get

perllsp provides:

| Feature | What It Does |
|---------|--------------|
| **Diagnostics** | Real-time syntax error detection |
| **Completion** | Variables, functions, keywords, file paths |
| **Hover** | Documentation for Perl built-ins, keywords, and special variables |
| **Definition** | Jump to where symbols are defined |
| **References** | Find all uses of a symbol |
| **Rename** | Safely rename variables across files |
| **Formatting** | Format code with Perl::Tidy |
| **Folding** | Collapse functions, blocks, POD |
| **Symbols** | Document outline and workspace search |

## Project Configuration

perllsp supports two ways to configure your project: a **project configuration file** for team-wide defaults, and **LSP settings** for personal or editor-specific overrides.

### Project Configuration File (.perl-lsp.toml)

The `.perl-lsp.toml` file lives at your workspace root and is committed to version control. It lets you share configuration with your whole team without requiring each developer to configure their own editor. The file is optional — if it does not exist, the server uses its built-in defaults.

Create a `.perl-lsp.toml` in the root of your project:

```toml
# .perl-lsp.toml — project-wide defaults for perl-lsp

[perl]
# Perl version hint (for future use)
version = "5.38"

# Module include paths relative to workspace root
include_paths = ["lib", "local/lib/perl5"]

[diagnostics]
# Enable perlcritic linting (requires perlcritic installed)
perlcritic = false
perlcritic_severity = 3

[features]
# Toggle all inlay hints
inlay_hints = true
```

**Key behaviors:**
- If the file does not exist, the server starts normally with defaults.
- Unknown keys and sections are silently ignored — safe to add future fields.
- Invalid TOML produces a warning notification in your editor.
- An empty `include_paths = []` is treated as "not set" and leaves the defaults unchanged.

A ready-to-copy example is available at [`.perl-lsp.toml.example`](../../.perl-lsp.toml.example) in the repo root.

### Configuration Precedence

Settings are applied in this order, last-write-wins:

```
.perl-lsp.toml  →  initializationOptions  →  didChangeConfiguration
(project file)      (editor startup)           (live editor settings)
```

Editor settings always override the project file. This lets individual developers override team defaults locally.

### LSP Settings (Editor-Specific)

For per-developer or editor-specific settings, configure via your editor's LSP mechanism.

#### Example: Configure Module Search Paths

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"]
    }
  }
}
```

#### Example: Tune for Large Projects

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

See [CONFIG.md](../reference/CONFIG.md) for all configuration options, including workspace paths, inlay hints, test-runner settings, and resource limits.

## Troubleshooting

Quick fixes for the most common first-run problems. For the full guide, see [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).

### "Binary not found" after install

`cargo install` places the binary in `~/.cargo/bin/`. If your shell cannot find `perllsp`, that directory is not on your `PATH`.

```bash
# Check whether the binary exists
ls ~/.cargo/bin/perllsp

# Add Cargo's bin directory to your PATH (add to ~/.bashrc, ~/.zshrc, or equivalent)
export PATH="$HOME/.cargo/bin:$PATH"

# Reload your shell
source ~/.bashrc   # or: source ~/.zshrc
```

After reloading, `perllsp --version` should print the version number.

### Extension / editor not connecting to the server

The editor must be able to find and launch the `perllsp` binary. Symptoms include "server failed to start" messages or LSP features simply not appearing.

1. **Verify the binary path** -- run `which perllsp` in the same shell your editor uses. Some editors (VS Code, for instance) may not inherit your shell's `PATH` when launched from a desktop shortcut. Try launching the editor from the terminal (`code .`) so it inherits your environment.

2. **Check editor logs** -- every LSP client has a log output:
   - VS Code: View > Output > select "Perl Language Server"
   - Neovim: `:LspLog`
   - Emacs: `*eglot stderr*` buffer

3. **Test JSON-RPC communication** manually:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perllsp --stdio
   ```
   You should see a JSON response. If you see an error, the binary itself has a problem -- try reinstalling.

4. **VS Code specific**: ensure the extension is installed and enabled:
   ```bash
   code --list-extensions | grep perl
   ```

### Completion not working

1. **Check file type registration** -- your editor must recognize the file as Perl. In VS Code, look at the language indicator in the bottom-right of the status bar (it should say "Perl"). In Neovim, run `:set filetype?` and confirm it says `filetype=perl`. Files without a `.pl`, `.pm`, or `.t` extension may not be detected automatically.

2. **Trigger completion manually** to rule out trigger-character issues:
   - VS Code: `Ctrl+Space`
   - Neovim: `<C-x><C-o>` (omni-completion) or use a completion plugin like nvim-cmp
   - Emacs: `M-TAB` or `C-M-i`

3. **Ensure the server is actually running** -- check `:LspInfo` (Neovim) or the Output panel (VS Code). If no server is attached, see the "Extension not connecting" section above.

### Tests are flaky when developing perl-lsp

If you are building perl-lsp from source and encounter intermittent test failures (particularly in LSP integration tests), constrain the thread count:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

The LSP integration tests start real server instances that compete for ports and file handles. Limiting parallelism eliminates the race conditions. See [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md) for more details on test threading.

### Server Not Starting

```bash
# Quick health check
perllsp --health

# Run with debug logging to see what's happening
RUST_LOG=perl_lsp=debug perllsp --stdio 2>debug.log
```

### No Diagnostics Appearing

1. Ensure your file has a Perl extension (`.pl`, `.pm`, `.t`)
2. Check your editor's language mode is set to Perl
3. Look at the LSP output log in your editor

### Slow on Large Projects

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

For the full troubleshooting guide including DAP debugging, parser edge cases, and editor-specific issues, see [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).

## Next Steps

- **[EDITOR_SETUP.md](../how-to/EDITOR_SETUP.md)** - Detailed editor configurations
- **[INSTALLATION.md](../how-to/INSTALLATION.md)** - Platform-specific installation and verification steps
- **[CONFIG.md](../reference/CONFIG.md)** - All configuration options
- **[LSP_FEATURES.md](../reference/LSP_FEATURES.md)** - Complete feature documentation
- **[FAQ.md](../reference/FAQ.md)** - Frequently asked questions
- **[Documentation Index](../INDEX.md)** - Documentation front door and routing guide

## Getting Help

- **Issues**: [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Documentation**: [docs/INDEX.md](../INDEX.md)
