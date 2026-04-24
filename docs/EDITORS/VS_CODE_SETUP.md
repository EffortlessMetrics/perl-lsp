# VS Code Setup Guide for perl-lsp

This guide helps you set up and configure the Perl Language Server in Visual Studio Code.

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

- **VS Code** version 1.88 or later
- **perl-lsp** server installed (see [Installation](#installation))

### Optional but Recommended

- **Perl** 5.10 or later (for syntax validation)
- **perltidy** (for code formatting)

---

## Installation

### Install the Server

Choose one of the following methods:

#### Option 1: Install from crates.io (Recommended)

```bash
cargo install perllsp
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
cd perl-lsp
cargo install perllsp
```

### Verify Installation

```bash
# Check version
perllsp --version

# Quick health check
perllsp --health
```

---

## Extension Setup

### Option 1: Official Extension (Recommended)

The official perl-lsp extension provides the best experience with automatic configuration.

```bash
# Install from command line
code --install-extension EffortlessMetrics.perl-lsp-rs

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

The extension exposes settings in the `perl-lsp.*` namespace. A separate `perl.*` namespace is used for server-side initialization options (see [Advanced Configuration](#advanced-configuration)).

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
  "perl-lsp.formatOnSave": true,
  "[perl]": {
    "editor.defaultFormatter": "EffortlessMetrics.perl-lsp-rs",
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

### All Extension Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `perl-lsp.serverPath` | string | `""` | Absolute path to `perl-lsp` binary. Leave empty to auto-download. |
| `perl-lsp.autoDownload` | boolean | `true` | Auto-download `perl-lsp` binary if not found locally. |
| `perl-lsp.includePaths` | array | `["lib", "local/lib/perl5"]` | Additional library paths to search for Perl modules. |
| `perl-lsp.enableDiagnostics` | boolean | `true` | Enable real-time syntax diagnostics. |
| `perl-lsp.enableSemanticTokens` | boolean | `true` | Enable semantic syntax highlighting. |
| `perl-lsp.perltidyConfig` | string | `""` | Path to `.perltidyrc` configuration file. |
| `perl-lsp.enableFormatting` | boolean | `true` | Enable document formatting using `perltidy`. |
| `perl-lsp.formatOnSave` | boolean | `false` | Format document on save. |
| `perl-lsp.enableRefactoring` | boolean | `true` | Enable refactoring-related code actions. |
| `perl-lsp.enableTestIntegration` | boolean | `true` | Enable `Test::More` and `Test2` integration. |
| `perl-lsp.autoPopulateNewFiles` | boolean | `true` | Auto-populate new `.pm` and `.t` files with boilerplate. |
| `perl-lsp.featureProfile` | string | `"auto"` | Runtime feature profile: `auto`, `ga`, `ga-lock`, `prod`, `all`. |
| `perl-lsp.trace.server` | string | `"off"` | LSP traffic logging: `off`, `messages`, `verbose`. |
| `perl-lsp.channel` | string | `"latest"` | Release channel: `latest`, `stable`, or `tag`. |
| `perl-lsp.versionTag` | string | `""` | Specific release tag when channel is `tag`. |
| `perl-lsp.downloadBaseUrl` | string | `""` | Internal base URL for hosting perl-lsp archives (bypasses GitHub). |

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

- **Keyboard**: `Ctrl+K Ctrl+I` or hover with mouse
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

- **Keyboard**: `Ctrl+.` (Cmd+. on macOS)
- **Context Menu**: Right-click → "Quick Fix"

Available actions:
- Extract variable
- Extract subroutine
- Organize imports

### Document Symbols

Navigate symbols in the current file:

- **Keyboard**: `Ctrl+Shift+O` (Cmd+Shift+O on macOS)
- **View**: Outline panel

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
- **On Save**: Enable with `perl-lsp.formatOnSave`

### Test Integration

Run tests directly from VS Code:

- **Keyboard**: `Shift+Alt+T`
- **Command Palette**: "Perl: Run Tests in Current File"
- **Editor toolbar**: Click the beaker icon on `.t` or `.pl` files

### Code Lens

Reference counts and quick actions inline in the editor.

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
| Open Symbol by Name | `Ctrl+T` | `Cmd+T` |
| Show All Symbols | `Ctrl+Shift+O` | `Cmd+Shift+O` |

### Extension-Specific Keybindings

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Run Tests | `Shift+Alt+T` | `Shift+Option+T` |
| Restart Server | `Shift+Alt+R` | `Shift+Option+R` |
| Organize Imports | `Shift+Alt+O` | `Shift+Option+O` |
| Extract Variable | `Shift+Alt+V` | `Shift+Option+V` |
| Extract Method | `Shift+Alt+M` | `Shift+Option+M` |

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

4. **Run health check**:
   - Press `Ctrl+Shift+P` → "Perl: Run Health Check"

5. **Test server manually**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perllsp --stdio
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

1. **Reduce result caps** (server-side limits via `initializationOptions`):
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

2. **Disable semantic tokens** (if not needed):
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

4. **Set perltidy config path** (optional):
   ```json
   {
     "perl-lsp.perltidyConfig": "/path/to/.perltidyrc"
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
  ]
}
```

### Feature Profile

Control which LSP features are active:

```json
{
  "perl-lsp.featureProfile": "ga"
}
```

Available profiles:
- `auto` (default) — follows the server binary build mode
- `ga-lock` — GA features only, no experimental
- `ga` — general availability features
- `prod` / `production` — alias for `ga`
- `all` — all features including experimental

### Release Channel

Pin to a specific release or use a different download channel:

```json
{
  "perl-lsp.channel": "tag",
  "perl-lsp.versionTag": "v0.12.0"
}
```

Available channels: `latest`, `stable`, `tag`.

### Internal Deployment

For teams hosting their own perl-lsp binaries:

```json
{
  "perl-lsp.serverPath": "/opt/perl-lsp/bin/perl-lsp",
  "perl-lsp.autoDownload": false
}
```

Or with an internal download mirror:

```json
{
  "perl-lsp.downloadBaseUrl": "https://internal.example.com/perl-lsp/"
}
```

### Debug Adapter Protocol (DAP)

Enable debugging support by creating a launch configuration. Run the command:

- Press `Ctrl+Shift+P` → "Perl: Create Debug Configuration"

Or add manually to `.vscode/launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Perl: Launch Script",
      "program": "${workspaceFolder}/script.pl",
      "stopOnEntry": true
    }
  ]
}
```

See [DAP User Guide](../tutorials/DAP_USER_GUIDE.md) for more details.

### Server-Side Performance Limits

The `perl.*` namespace passes server-side initialization options. These control internal server caps and are separate from the extension's `perl-lsp.*` settings:

```json
{
  "perl": {
    "limits": {
      "workspaceSymbolCap": 200,
      "referencesCap": 500,
      "completionCap": 100,
      "astCacheMaxEntries": 50,
      "maxIndexedFiles": 5000,
      "maxTotalSymbols": 250000,
      "workspaceScanDeadlineMs": 20000,
      "referenceSearchDeadlineMs": 1500
    }
  }
}
```

### Logging and Tracing

Enable detailed logging for troubleshooting:

```json
{
  "perl-lsp.trace.server": "verbose"
}
```

Logs appear in the VS Code Output panel under "Perl Language Server".

---

## Complete Example Configuration

Here is a typical `.vscode/settings.json` for a Perl project using only real extension settings:

```json
{
  "perl-lsp.serverPath": "",
  "perl-lsp.autoDownload": true,
  "perl-lsp.trace.server": "off",
  "perl-lsp.enableDiagnostics": true,
  "perl-lsp.enableSemanticTokens": true,
  "perl-lsp.enableFormatting": true,
  "perl-lsp.formatOnSave": true,
  "perl-lsp.enableRefactoring": true,
  "perl-lsp.enableTestIntegration": true,
  "perl-lsp.includePaths": [
    "lib",
    "local/lib/perl5",
    "vendor/lib"
  ],
  "perl-lsp.perltidyConfig": "",
  "perl-lsp.autoPopulateNewFiles": true,
  "perl-lsp.featureProfile": "auto",

  "[perl]": {
    "editor.defaultFormatter": "EffortlessMetrics.perl-lsp-rs",
    "editor.formatOnSave": true,
    "editor.tabSize": 4,
    "editor.insertSpaces": true
  },

  "files.exclude": {
    "**/.git": true,
    "**/.DS_Store": true,
    "**/node_modules": true
  },

  "search.exclude": {
    "**/node_modules": true,
    "**/local": true
  }
}
```

---

## See Also

- [Getting Started](../tutorials/GETTING_STARTED.md) - Quick start guide
- [Configuration Reference](../reference/CONFIG.md) - Complete configuration options
- [Troubleshooting Guide](../how-to/TROUBLESHOOTING.md) - Common issues and solutions
- [DAP User Guide](../tutorials/DAP_USER_GUIDE.md) - Debugging setup
- [Editor Setup](../how-to/EDITOR_SETUP.md) - Other editor configurations
