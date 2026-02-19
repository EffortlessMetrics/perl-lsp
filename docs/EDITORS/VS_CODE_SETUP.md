# VS Code Setup Guide for perl-lsp

This comprehensive guide helps you set up and configure the Perl Language Server in Visual Studio Code.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Extension Setup](#extension-setup)
- [Configuration](#configuration)
- [Features](#features)
- [Keybindings](#keybindings)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

---

## Prerequisites

### Required

- **VS Code** version 1.80 or later
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

Download from [GitHub Releases](https://github.com/EffortlessMetrics/perl-lsp/releases):

```bash
# Linux (x86_64)
curl -LO https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-linux-x86_64.tar.gz
tar xzf perl-lsp-linux-x86_64.tar.gz
sudo mv perl-lsp /usr/local/bin/

# macOS (Apple Silicon)
curl -LO https://github.com/EffortlessMetrics/perl-lsp/releases/latest/download/perl-lsp-darwin-aarch64.tar.gz
tar xzf perl-lsp-darwin-aarch64.tar.gz
sudo mv perl-lsp /usr/local/bin/

# Windows (x86_64)
# Download and extract to a directory in your PATH
```

#### Option 3: Build from Source

```bash
git clone https://github.com/EffortlessMetrics/perl-lsp.git
cd tree-sitter-perl-rs
cargo install --path crates/perl-lsp
```

### Verify Installation

```bash
# Check version
perl-lsp --version

# Quick health check
perl-lsp --health
# Should output: ok 0.9.1
```

---

## Extension Setup

### Option 1: Official Extension (Recommended)

The official perl-lsp extension provides the best experience with automatic configuration.

```bash
# Install from command line
code --install-extension effortlesssteven.perl-lsp

# Or search in VS Code Extensions marketplace:
# 1. Press Ctrl+Shift+X (Cmd+Shift+X on macOS)
# 2. Search for "perl-lsp"
# 3. Click "Install"
```

### Option 2: Generic LSP Client

If you prefer using a generic LSP client extension:

1. Install the [Generic LSP Client](https://marketplace.visualstudio.com/items?itemName=matthewbystrom.genericlspclient) extension
2. Configure as shown below

---

## Configuration

### Basic Configuration

Add to your workspace `.vscode/settings.json`:

```json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": false,
  "perl-lsp.enableRefactoring": true,
  "perl-lsp.includePaths": ["lib", "local/lib/perl5"],
  "perl-lsp.enableTestIntegration": true
}
```

### Workspace-Specific Configuration

For project-specific settings, create `.vscode/settings.json` in your project root:

```json
{
  "perl-lsp.includePaths": [
    "lib",
    "local/lib/perl5",
    "vendor/lib"
  ],
  "perl-lsp.useSystemInc": false,
  "perl-lsp.formatOnSave": true,
  "[perl]": {
    "editor.defaultFormatter": "effortlesssteven.perl-lsp",
    "editor.formatOnSave": true
  }
}
```

### User-Level Configuration

For global settings, open VS Code settings (`Ctrl+,` or `Cmd+,`):

1. Search for "perl-lsp"
2. Configure settings as needed

Or edit `settings.json` directly:

1. Press `Ctrl+Shift+P` (Cmd+Shift+P on macOS)
2. Type "Preferences: Open Settings (JSON)"
3. Add your configuration

---

## Features

### Syntax Diagnostics

Real-time syntax error detection and reporting:

```perl
# Errors are highlighted as you type
my $x = 1
# Missing semicolon - error shown immediately
```

### Go to Definition

Navigate to symbol definitions:

- **Keyboard**: `F12` or `Ctrl+Click` (Cmd+Click on macOS)
- **Context Menu**: Right-click → "Go to Definition"

```perl
use MyModule;

MyModule::some_function();
# ^ F12 here jumps to the definition
```

### Find References

Find all usages of a symbol:

- **Keyboard**: `Shift+F12`
- **Context Menu**: Right-click → "Find All References"

```perl
sub my_function {
    return 42;
}

# ^ Find references here shows all calls to my_function
```

### Hover Information

View documentation and type information:

- **Keyboard**: `Ctrl+Space` or hover with mouse
- **Shows**: Function signatures, variable types, documentation

### Code Completion

Intelligent code completion:

- **Keyboard**: `Ctrl+Space`
- **Triggers**: Automatically as you type

```perl
use MyModule;

MyModule::  # Press Ctrl+Space for completion
```

### Semantic Highlighting

Enhanced syntax highlighting based on semantic understanding:

- Variables, functions, types are color-coded
- Comments and strings are properly highlighted
- Special Perl constructs are highlighted

### Code Actions

Quick fixes and refactorings:

- **Keyboard**: `Ctrl+.` (Cmd+.` on macOS)
- **Context Menu**: Right-click → "Quick Fix"

Available actions:
- Extract variable
- Extract subroutine
- Optimize imports
- Add missing pragmas

### Document Symbols

Navigate symbols in the current file:

- **Keyboard**: `Ctrl+Shift+O` (Cmd+Shift+O on macOS)
- **View**: Outline panel (Ctrl+Shift+B)

### Workspace Symbols

Search symbols across the entire workspace:

- **Keyboard**: `Ctrl+T` (Cmd+T on macOS)
- **Search**: Type symbol name to find it

### Rename Symbol

Rename symbols across the workspace:

- **Keyboard**: `F2`
- **Context Menu**: Right-click → "Rename Symbol"

### Formatting

Format Perl code using perltidy:

- **Keyboard**: `Shift+Alt+F` (Shift+Option+F on macOS)
- **Command**: Format Document
- **On Save**: Enable with `formatOnSave` setting

### Inlay Hints

Inline type and parameter hints:

```perl
sub my_function($name, $count) {
    return "Hello, $name x$count";
}

my_function("World", 5);
# ^ Shows: my_function(/* name: */ "World", /* count: */ 5)
```

Enable in settings:

```json
{
  "perl-lsp.inlayHints.enabled": true,
  "perl-lsp.inlayHints.parameterHints": true,
  "perl-lsp.inlayHints.typeHints": true
}
```

### Test Integration

Run tests directly from VS Code:

```json
{
  "perl-lsp.testRunner.enabled": true,
  "perl-lsp.testRunner.command": "prove",
  "perl-lsp.testRunner.args": ["-l", "-v"]
}
```

### Code Lens

Reference counts and quick actions:

```perl
sub my_function {
    # ^ Shows: 3 references
}
```

---

## Keybindings

### Default LSP Keybindings

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Go to Definition | `F12` | `F12` |
| Peek Definition | `Ctrl+Shift+F10` | `Ctrl+Shift+F10` |
| Find References | `Shift+F12` | `Shift+F12` |
| Rename Symbol | `F2` | `F2` |
| Format Document | `Shift+Alt+F` | `Shift+Option+F` |
| Quick Fix | `Ctrl+.` | `Cmd+.` |
| Show Hover | `Ctrl+K Ctrl+I` | `Ctrl+K Ctrl+I` |
| Go to References | `Shift+F12` | `Shift+F12` |
| Go to Implementation | `Ctrl+F12` | `Ctrl+F12` |
| Open Symbol by Name | `Ctrl+T` | `Cmd+T` |
| Show All Symbols | `Ctrl+Shift+O` | `Cmd+Shift+O` |

### Custom Keybindings

To customize keybindings, edit `keybindings.json`:

1. Press `Ctrl+Shift+P` (Cmd+Shift+P on macOS)
2. Type "Preferences: Open Keyboard Shortcuts (JSON)"
3. Add custom bindings

Example:

```json
[
  {
    "key": "ctrl+shift+r",
    "command": "editor.action.rename",
    "when": "editorHasRenameProvider && editorTextFocus"
  },
  {
    "key": "ctrl+shift+f",
    "command": "editor.action.formatDocument",
    "when": "editorHasDocumentFormattingProvider && editorTextFocus && !editorReadonly"
  }
]
```

---

## Troubleshooting

### Server Not Starting

**Symptoms**: No diagnostics, no completion, error in output panel

**Solutions**:

1. **Verify binary is in PATH**:
   ```bash
   which perl-lsp
   # Should output: /usr/local/bin/perl-lsp or similar
   ```

2. **Check extension logs**:
   - Open Output panel: `Ctrl+Shift+U` (Cmd+Shift+U on macOS)
   - Select "Perl Language Server" from dropdown
   - Look for error messages

3. **Enable debug logging**:
   ```json
   {
     "perl-lsp.trace.server": "verbose"
   }
   ```

4. **Test server manually**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

### No Diagnostics

**Symptoms**: No errors shown for invalid code

**Solutions**:

1. **Check file type**:
   - Ensure file has `.pl`, `.pm`, or `.t` extension
   - Check language mode: Click language indicator in status bar → select "Perl"

2. **Verify diagnostics enabled**:
   ```json
   {
     "perl-lsp.enableDiagnostics": true
   }
   ```

3. **Check for syntax errors in configuration**:
   - Open Output panel → "Perl Language Server"
   - Look for configuration errors

### Slow Performance

**Symptoms**: Lag when typing, slow completions

**Solutions**:

1. **Reduce result caps**:
   ```json
   {
     "perl": {
       "limits": {
         "workspaceSymbolCap": 100,
         "referencesCap": 200,
         "completionCap": 50
       }
     }
   }
   ```

2. **Disable system @INC**:
   ```json
   {
     "perl-lsp.useSystemInc": false
   }
   ```

3. **Reduce resolution timeout**:
   ```json
   {
     "perl-lsp.resolutionTimeout": 25
   }
   ```

4. **Disable semantic tokens** (if not needed):
   ```json
   {
     "perl-lsp.enableSemanticTokens": false
   }
   ```

### Module Resolution Issues

**Symptoms**: Can't find modules, go-to-definition fails

**Solutions**:

1. **Check include paths**:
   ```json
   {
     "perl-lsp.includePaths": ["lib", ".", "local/lib/perl5", "vendor/lib"]
   }
   ```

2. **Verify module exists**:
   ```bash
   perl -e 'use Module::Name;'
   ```

3. **Check workspace root**:
   - Ensure VS Code opened the correct project folder
   - Right-click folder → "Open Folder"

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

   # Windows (via Strawberry Perl)
   ppm install Perl-Tidy
   ```

2. **Check perltidy works**:
   ```bash
   perltidy --version
   ```

3. **Verify formatting enabled**:
   ```json
   {
     "perl-lsp.enableFormatting": true,
     "perl-lsp.formatOnSave": true
   }
   ```

### Extension Conflicts

**Symptoms**: Duplicate diagnostics, conflicting keybindings

**Solutions**:

1. **Disable other Perl extensions**:
   - Open Extensions panel: `Ctrl+Shift+X` (Cmd+Shift+X on macOS)
   - Search for "perl"
   - Disable extensions that might conflict (e.g., other LSP servers)

2. **Check for duplicate language servers**:
   - Open Output panel → "Perl Language Server"
   - Look for messages about multiple servers

---

## Advanced Configuration

### Multi-Root Workspace

For workspaces with multiple folders:

```json
{
  "perl-lsp.includePaths": [
    "${workspaceFolder}/lib",
    "${workspaceFolder}/local/lib/perl5"
  ],
  "perl-lsp.useSystemInc": false
}
```

### Environment Variables

Set environment variables for the LSP server:

```json
{
  "perl-lsp.env": {
    "PERL5LIB": "${workspaceFolder}/lib",
    "PERL_MB_OPT": "--install_base ${workspaceFolder}/local"
  }
}
```

### Custom Formatter

Use a custom formatter instead of perltidy:

```json
{
  "perl-lsp.formatting.provider": "custom",
  "perl-lsp.formatting.command": "my-formatter",
  "perl-lsp.formatting.args": ["--style", "perl"]
}
```

### Debug Adapter Protocol (DAP)

Enable debugging support:

```json
{
  "perl-lsp.enableDebugAdapter": true,
  "perl-lsp.debugAdapter.port": 9257
}
```

See [DAP User Guide](../DAP_USER_GUIDE.md) for more details.

### Workspace Folders

Configure workspace-specific settings:

```json
{
  "perl-lsp.workspaceFolders": [
    {
      "uri": "file:///path/to/project1",
      "name": "Project 1",
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["lib", "local/lib/perl5"]
          }
        }
      }
    },
    {
      "uri": "file:///path/to/project2",
      "name": "Project 2",
      "settings": {
        "perl": {
          "workspace": {
            "includePaths": ["src"]
          }
        }
      }
    }
  ]
}
```

### Performance Tuning

For large workspaces, adjust performance settings:

```json
{
  "perl": {
    "limits": {
      "workspaceSymbolCap": 100,
      "referencesCap": 200,
      "completionCap": 50,
      "astCacheMaxEntries": 50,
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 250000,
      "workspaceScanDeadlineMs": 20000,
      "referenceSearchDeadlineMs": 1500
    },
    "workspace": {
      "resolutionTimeout": 25
    }
  }
}
```

### Logging

Enable detailed logging for troubleshooting:

```json
{
  "perl-lsp.trace.server": "verbose",
  "perl-lsp.logLevel": "debug"
}
```

Logs are written to:
- **Windows**: `%APPDATA%\Code\logs\`
- **macOS**: `~/Library/Logs/Code/`
- **Linux**: `~/.config/Code/logs/`

---

## Complete Example Configuration

Here's a comprehensive example configuration for a typical Perl project:

```json
{
  // perl-lsp extension settings
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": true,
  "perl-lsp.enableRefactoring": true,
  "perl-lsp.enableTestIntegration": true,

  // Workspace configuration
  "perl-lsp.includePaths": [
    "lib",
    "local/lib/perl5",
    "vendor/lib"
  ],
  "perl-lsp.useSystemInc": false,
  "perl-lsp.resolutionTimeout": 50,

  // Inlay hints
  "perl-lsp.inlayHints.enabled": true,
  "perl-lsp.inlayHints.parameterHints": true,
  "perl-lsp.inlayHints.typeHints": true,
  "perl-lsp.inlayHints.maxLength": 30,

  // Test runner
  "perl-lsp.testRunner.enabled": true,
  "perl-lsp.testRunner.command": "prove",
  "perl-lsp.testRunner.args": ["-l", "-v"],
  "perl-lsp.testRunner.timeout": 60000,

  // Performance limits
  "perl-lsp.workspaceSymbolCap": 200,
  "perl-lsp.referencesCap": 500,
  "perl-lsp.completionCap": 100,

  // Language-specific settings
  "[perl]": {
    "editor.defaultFormatter": "effortlesssteven.perl-lsp",
    "editor.formatOnSave": true,
    "editor.tabSize": 4,
    "editor.insertSpaces": true,
    "editor.wordBasedSuggestions": true
  },

  // Files to exclude
  "files.exclude": {
    "**/.git": true,
    "**/.svn": true,
    "**/.hg": true,
    "**/CVS": true,
    "**/.DS_Store": true,
    "**/node_modules": true
  },

  // Search exclusions
  "search.exclude": {
    "**/node_modules": true,
    "**/bower_components": true,
    "**/local": true
  }
}
```

---

## See Also

- [Getting Started](../GETTING_STARTED.md) - Quick start guide
- [Configuration Reference](../CONFIG.md) - Complete configuration options
- [Troubleshooting Guide](../TROUBLESHOOTING.md) - Common issues and solutions
- [Performance Tuning](../PERFORMANCE_TUNING.md) - Performance optimization guide
- [DAP User Guide](../DAP_USER_GUIDE.md) - Debugging setup
- [Editor Setup](../EDITOR_SETUP.md) - Other editor configurations
