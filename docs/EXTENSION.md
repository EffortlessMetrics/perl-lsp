# VSCode Extension Guide

## Installation

### From VSCode Marketplace
1. Open VSCode
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "Perl Language Server"
4. Click Install

### From Open VSX (for VSCodium, Gitpod, etc.)
```bash
code --install-extension tree-sitter-perl.perl-lsp
```

### Manual Installation
Download the `.vsix` file from [releases](https://github.com/EffortlessMetrics/perl-lsp/releases) and install:
```bash
code --install-extension perl-lsp-*.vsix
```

## Configuration

### Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `perl-lsp.autoDownload` | `true` | Automatically download LSP binary if not found |
| `perl-lsp.serverPath` | `"perl-lsp"` | Path to the LSP server binary |
| `perl-lsp.trace.server` | `"off"` | LSP communication tracing |
| `perl-lsp.maxNumberOfProblems` | `100` | Maximum diagnostics per file |
| `perl-lsp.enableStrictMode` | `false` | Enable strict mode warnings |
| `perl-lsp.enableWarnings` | `true` | Enable Perl warnings |

### Custom Server Path
If you have a custom build or prefer a specific version:
```json
{
  "perl-lsp.serverPath": "/usr/local/bin/perl-lsp",
  "perl-lsp.autoDownload": false
}
```

## Features

### Auto-Download
The extension automatically downloads the correct LSP binary for your platform:
- Detects OS and architecture (Windows/macOS/Linux, x64/arm64)
- Downloads from GitHub releases with SHA256 verification
- Caches in extension storage
- Shows progress notifications

### Supported Features
- ✅ **Diagnostics**: Real-time syntax and semantic errors
- ✅ **Completion**: Variables, functions, keywords, modules
- ✅ **Go to Definition**: Jump to declarations
- ✅ **Find References**: Find all usages
- ✅ **Hover**: Documentation and type info
- ✅ **Signature Help**: Parameter hints
- ✅ **Document Symbols**: Outline view
- ✅ **Rename**: Safe refactoring
- ✅ **Code Actions**: Quick fixes and refactorings
- ✅ **Semantic Highlighting**: Smart syntax coloring

## Troubleshooting

### Server Not Starting
1. Check Output panel (View → Output → Perl LSP)
2. Verify server path: `which perl-lsp`
3. Try manual download: [releases](https://github.com/EffortlessMetrics/perl-lsp/releases)
4. Disable auto-download and set custom path

### CRLF Line Endings
The LSP handles CRLF correctly but some edge cases exist:
- Position mapping for `\r` in CRLF may be off by 1
- Workaround: Save files with LF endings on Unix systems

### Performance Issues
- Increase `perl-lsp.maxNumberOfProblems` for large files
- Check server logs with `"perl-lsp.trace.server": "verbose"`

### Offline Installation
1. Download the server binary from GitHub releases
2. Download the `.vsix` extension file
3. Install extension: `code --install-extension perl-lsp-*.vsix`
4. Configure server path in settings

## Commands

| Command | Description | Shortcut |
|---------|-------------|----------|
| `Perl: Restart Language Server` | Restart the LSP server | - |
| `Perl: Show Output` | Show LSP output channel | - |
| `Perl: Run Tests` | Run Perl tests | - |

## Development

### Building from Source
```bash
# Clone repository
git clone https://github.com/EffortlessMetrics/perl-lsp
cd tree-sitter-perl/vscode-extension

# Install dependencies
npm install

# Compile
npm run compile

# Package
vsce package
```

### Testing Locally
1. Open `vscode-extension` folder in VSCode
2. Press F5 to launch Extension Development Host
3. Open a Perl file to test

## Support

- **Issues**: [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues)
- **Discussions**: [GitHub Discussions](https://github.com/EffortlessMetrics/perl-lsp/discussions)
- **Updates**: Watch the repository for releases