# Helix Setup Guide for perl-lsp

This comprehensive guide helps you set up and configure the Perl Language Server in Helix editor.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Basic Setup](#basic-setup)
- [Configuration](#configuration)
- [Features](#features)
- [Keybindings](#keybindings)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

---

## Prerequisites

### Required

- **Helix** version 23.03 or later
- **perl-lsp** server installed (see [Installation](#installation))

### Optional but Recommended

- **Perl** 5.10 or later (for syntax validation)
- **perltidy** (for code formatting)
- **perlcritic** (for linting)

---

## Installation

### Install the Server

Choose one of the following methods:

#### Option 1: Install from crates.io (Recommended)

```bash
cargo install perl-lsp
```

#### Option 2: Download Pre-built Binary

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

#### Option 3: Build from Source

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

## Basic Setup

### Configuration File

Helix uses TOML configuration files located at:

- **Linux/macOS**: `~/.config/helix/languages.toml`
- **Windows**: `%APPDATA%\helix\languages.toml`

### Basic Configuration

Add the following to your `languages.toml`:

```toml
[[language]]
name = "perl"
scope = "source.perl"
injection-regex = "perl"
file-types = ["pl", "pm", "t", "psgi"]
roots = ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"]
comment-token = "#"
indent = { tab-width = 4, unit = "    " }
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]
```

### Verify Setup

1. Restart Helix
2. Open a `.pl` or `.pm` file
3. Check if LSP is attached:
   - Press `:`
   - Type `lsp`
   - Look for perl-lsp in the list

---

## Configuration

### Full Configuration

Add to your `languages.toml`:

```toml
[[language]]
name = "perl"
scope = "source.perl"
injection-regex = "perl"
file-types = ["pl", "pm", "t", "psgi"]
roots = ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"]
comment-token = "#"
indent = { tab-width = 4, unit = "    " }
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]

[language-server.perl-lsp.config.perl]
workspace.includePaths = ["lib", ".", "local/lib/perl5"]
workspace.useSystemInc = false
workspace.resolutionTimeout = 50
inlayHints.enabled = true
inlayHints.parameterHints = true
inlayHints.typeHints = true
inlayHints.maxLength = 30
testRunner.enabled = true
testRunner.command = "perl"
testRunner.args = []
testRunner.timeout = 60000
limits.workspaceSymbolCap = 200
limits.referencesCap = 500
limits.completionCap = 100
limits.astCacheMaxEntries = 100
limits.maxIndexedFiles = 10000
limits.maxTotalSymbols = 500000
limits.workspaceScanDeadlineMs = 30000
limits.referenceSearchDeadlineMs = 2000
```

### Project-Specific Configuration

Create `.helix/languages.toml` in your project root:

```toml
[[language]]
name = "perl"
language-servers = ["perl-lsp"]

[language-server.perl-lsp.config.perl]
workspace.includePaths = ["lib", "local/lib/perl5", "vendor/lib"]
workspace.useSystemInc = false
```

---

## Features

### Syntax Diagnostics

Real-time syntax error detection and reporting:

```perl
# Errors are highlighted as you type
my $x = 1
# Missing semicolon - error shown immediately
```

View diagnostics:
- Press `:`
- Type `diagnostics`

### Go to Definition

Navigate to symbol definitions:

- **Keybinding**: `gd`
- **Command**: `:goto_definition`

```perl
use MyModule;

MyModule::some_function();
# ^ gd here jumps to the definition
```

### Find References

Find all usages of a symbol:

- **Keybinding**: `gr`
- **Command**: `:references`

```perl
sub my_function {
    return 42;
}

# ^ Find references here shows all calls to my_function
```

### Hover Information

View documentation and type information:

- **Keybinding**: `K`
- **Command**: `:hover`

### Code Completion

Intelligent code completion:

- **Keybinding**: `Ctrl+x Ctrl+o` or type to trigger
- **Command**: `:completion`

```perl
use MyModule;

MyModule::  # Type here for completion
```

### Document Symbols

Navigate symbols in the current file:

- **Keybinding**: `g a`
- **Command**: `:symbol-picker`

### Workspace Symbols

Search symbols across the entire workspace:

- **Keybinding**: `Space s s`
- **Command**: `:workspace_symbol`

### Rename Symbol

Rename symbols across the workspace:

- **Keybinding**: `Space r n`
- **Command**: `:rename`

### Formatting

Format Perl code using perltidy:

- **Keybinding**: `Space f`
- **Command**: `:format`

### Code Actions

Quick fixes and refactorings:

- **Keybinding**: `Space a a`
- **Command**: `:code_action`

Available actions:
- Extract variable
- Extract subroutine
- Optimize imports
- Add missing pragmas

### Inlay Hints

Inline type and parameter hints:

```perl
sub my_function($name, $count) {
    return "Hello, $name x$count";
}

my_function("World", 5);
# ^ Shows: my_function(/* name: */ "World", /* count: */ 5)
```

Enable inlay hints:
- **Keybinding**: `Space h i`
- **Command**: `:inlay_hints`

---

## Keybindings

### Default LSP Keybindings

| Action | Keybinding | Command |
|--------|------------|---------|
| Go to Definition | `gd` | `:goto_definition` |
| Find References | `gr` | `:references` |
| Hover | `K` | `:hover` |
| Completion | `Ctrl+x Ctrl+o` | `:completion` |
| Document Symbols | `g a` | `:symbol-picker` |
| Workspace Symbols | `Space s s` | `:workspace_symbol` |
| Rename | `Space r n` | `:rename` |
| Format | `Space f` | `:format` |
| Code Actions | `Space a a` | `:code_action` |
| Inlay Hints | `Space h i` | `:inlay_hints` |
| Diagnostics | `Space d d` | `:diagnostics` |
| Next Diagnostic | `] d` | - |
| Previous Diagnostic | `[ d` | - |

### Custom Keybindings

Add to your `config.toml`:

```toml
[keys.normal]
# LSP keybindings
gd = ":goto_definition"
gr = ":references"
K = ":hover"

# Space keybindings
[keys.normal.space]
f = ":format"
a = { a = ":code_action" }
r = { n = ":rename" }
s = { s = ":workspace_symbol" }
d = { d = ":diagnostics" }
h = { i = ":inlay_hints" }
```

---

## Troubleshooting

### Server Not Starting

**Symptoms**: No diagnostics, no completion, error in status bar

**Solutions**:

1. **Verify binary is in PATH**:
   ```bash
   which perl-lsp
   # Should output: /usr/local/bin/perl-lsp or similar
   ```

2. **Check LSP status**:
   - Press `:`
   - Type `lsp`
   - Look for perl-lsp in the list

3. **Check logs**:
   ```bash
   # Helix logs are in:
   # Linux: ~/.cache/helix/helix.log
   # macOS: ~/Library/Caches/helix/helix.log
   # Windows: %APPDATA%\helix\helix.log
   ```

4. **Test server manually**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

**Symptoms**: No errors shown for invalid code

**Solutions**:

1. **Check file type**:
   - Look at the status bar
   - Should show "perl" as the language

2. **Set file type manually**:
   - Press `:`
   - Type `set-language perl`

3. **Verify diagnostics enabled**:
   - Press `:`
   - Type `diagnostics`

### Slow Performance

**Symptoms**: Lag when typing, slow completions

**Solutions**:

1. **Reduce result caps**:
   ```toml
   [language-server.perl-lsp.config.perl]
   limits.workspaceSymbolCap = 100
   limits.referencesCap = 200
   limits.completionCap = 50
   ```

2. **Disable system @INC**:
   ```toml
   [language-server.perl-lsp.config.perl]
   workspace.useSystemInc = false
   ```

3. **Reduce resolution timeout**:
   ```toml
   [language-server.perl-lsp.config.perl]
   workspace.resolutionTimeout = 25
   ```

### Module Resolution Issues

**Symptoms**: Can't find modules, go-to-definition fails

**Solutions**:

1. **Check include paths**:
   ```toml
   [language-server.perl-lsp.config.perl]
   workspace.includePaths = ["lib", ".", "local/lib/perl5", "vendor/lib"]
   ```

2. **Verify module exists**:
   ```bash
   perl -e 'use Module::Name;'
   ```

3. **Check workspace root**:
   - Press `:`
   - Type `pwd`

### Formatting Not Working

**Symptoms**: Format command does nothing or errors

**Solutions**:

1. **Install perltidy**:
   ```bash
   # macOS
   brew install perltidy

   # Ubuntu/Debian
   sudo apt-get install perltidy

   # CentOS/RHEL
   sudo yum install perl-Perl-Tidy
   ```

2. **Check perltidy works**:
   ```bash
   perltidy --version
   ```

3. **Verify formatting enabled**:
   - Press `:`
   - Type `format`

### Tree-sitter Grammar Issues

**Symptoms**: Incorrect highlighting, parsing errors

**Solutions**:

1. **Install tree-sitter CLI**:
   ```bash
   cargo install tree-sitter-cli
   ```

2. **Fetch and build grammar**:
   ```bash
   cd ~/.local/share/helix/runtime
   hx --grammar fetch
   hx --grammar build
   ```

3. **Restart Helix**

---

## Advanced Configuration

### Multi-Root Workspace

For workspaces with multiple folders:

```toml
[[language]]
name = "perl"
roots = ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git", "Cargo.toml"]
```

### Environment Variables

Set environment variables for the LSP server:

```toml
[language-server.perl-lsp]
command = "env"
args = ["PERL5LIB=lib", "PERL_MB_OPT=--install_base local", "perl-lsp", "--stdio"]
```

### Custom Formatter

Use a custom formatter instead of perltidy:

```toml
[[language]]
name = "perl"
formatter = { command = "my-formatter", args = ["--style", "perl"] }
```

### Debug Adapter Protocol (DAP)

Enable debugging support:

```toml
[language.debugger]
name = "perl-debug"
command = "perl-debug-adapter"
args = []
transport = "tcp"
port-arg = "--port {}"
```

See [DAP User Guide](../DAP_USER_GUIDE.md) for more details.

### Performance Tuning

For large workspaces, adjust performance settings:

```toml
[language-server.perl-lsp.config.perl]
limits.workspaceSymbolCap = 100
limits.referencesCap = 200
limits.completionCap = 50
limits.astCacheMaxEntries = 50
limits.maxIndexedFiles = 5000
limits.maxTotalSymbols = 250000
limits.workspaceScanDeadlineMs = 20000
limits.referenceSearchDeadlineMs = 1500
workspace.resolutionTimeout = 25
```

### Auto-Save Configuration

Configure auto-save behavior:

```toml
[editor]
auto-save = true
auto-save-timeout = 1000
```

### Indentation Configuration

Customize indentation for Perl:

```toml
[[language]]
name = "perl"
indent = { tab-width = 4, unit = "    " }
```

### Comment Configuration

Customize comment behavior:

```toml
[[language]]
name = "perl"
comment-token = "#"
```

---

## Complete Example Configuration

Here's a comprehensive example configuration:

### languages.toml

```toml
[[language]]
name = "perl"
scope = "source.perl"
injection-regex = "perl"
file-types = ["pl", "pm", "t", "psgi"]
roots = ["Makefile.PL", "Build.PL", "cpanfile", "dist.ini", ".git"]
comment-token = "#"
indent = { tab-width = 4, unit = "    " }
language-servers = ["perl-lsp"]

[language-server.perl-lsp]
command = "perl-lsp"
args = ["--stdio"]

[language-server.perl-lsp.config.perl]
# Workspace configuration
workspace.includePaths = ["lib", ".", "local/lib/perl5", "vendor/lib"]
workspace.useSystemInc = false
workspace.resolutionTimeout = 50

# Inlay hints
inlayHints.enabled = true
inlayHints.parameterHints = true
inlayHints.typeHints = true
inlayHints.maxLength = 30

# Test runner
testRunner.enabled = true
testRunner.command = "perl"
testRunner.args = []
testRunner.timeout = 60000

# Performance limits
limits.workspaceSymbolCap = 200
limits.referencesCap = 500
limits.completionCap = 100
limits.astCacheMaxEntries = 100
limits.maxIndexedFiles = 10000
limits.maxTotalSymbols = 500000
limits.workspaceScanDeadlineMs = 30000
limits.referenceSearchDeadlineMs = 2000
```

### config.toml

```toml
[editor]
line-number = "relative"
mouse = true
auto-save = true
auto-save-timeout = 1000
completion-timeout = 250
idle-timeout = 250
preview-completion-insert = true

[editor.cursor-shape]
insert = "bar"
normal = "block"
select = "underline"

[editor.file-picker]
hidden = false
git-ignore = true
git-global = true
parents = true

[editor.soft-wrap]
enable = true
max-wrap = 25

[editor.whitespace]
render = "all"

[editor.indent-guides]
render = true

[keys.normal]
# LSP keybindings
gd = ":goto_definition"
gr = ":references"
K = ":hover"

# Space keybindings
[keys.normal.space]
f = ":format"
a = { a = ":code_action" }
r = { n = ":rename" }
s = { s = ":workspace_symbol" }
d = { d = ":diagnostics" }
h = { i = ":inlay_hints" }

# File operations
[keys.normal.space.f]
f = ":format"
s = ":write"

# Window management
[keys.normal.space.w]
h = ":hsplit"
v = ":vsplit"
c = ":buffer-close"

# Search
[keys.normal.space.s]
s = ":workspace_symbol"
f = ":global_search"

# Git
[keys.normal.space.g]
c = ":git-commit"
p = ":git-push"
l = ":git-log"
```

---

## See Also

- [Getting Started](../GETTING_STARTED.md) - Quick start guide
- [Configuration Reference](../CONFIG.md) - Complete configuration options
- [Troubleshooting Guide](../TROUBLESHOOTING.md) - Common issues and solutions
- [Performance Tuning](../PERFORMANCE_TUNING.md) - Performance optimization guide
- [Editor Setup](../EDITOR_SETUP.md) - Other editor configurations
- [Helix Documentation](https://docs.helix-editor.com/)
