# Perl LSP v0.8.3-rc1 Internal Release

## 🚀 Quick Start

### Installation (Linux/macOS)

```bash
# Extract the package
tar -xzf perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu.tar.gz

# Install globally (recommended)
sudo cp perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu/perl-lsp /usr/local/bin/
perl-lsp --version

# Or install to user directory
cp perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu/perl-lsp ~/.local/bin/
```

## 📦 Package Contents

- `perl-lsp` - The language server binary
- `README.md` - Basic documentation

## 🔧 VS Code Configuration

Since this is a private repository, GitHub releases are not available.
Configure VS Code to use the local binary:

### Option 1: Local Binary Path (Recommended)

Add to `.vscode/settings.json`:

```json
{
  "perl-lsp.serverPath": "/usr/local/bin/perl-lsp",
  "perl-lsp.autoDownload": false,
  "perl-lsp.channel": "tag",
  "perl-lsp.versionTag": "v0.8.3-rc1"
}
```

### Option 2: Team Configuration

For teams, add to workspace settings:

```json
{
  "perl-lsp.serverPath": "${env:PERL_LSP_PATH}",
  "perl-lsp.autoDownload": false
}
```

Then each team member sets: `export PERL_LSP_PATH=/usr/local/bin/perl-lsp`

## 📋 Features

### Language Server Protocol (v0.8.3)
- ✅ **30+ IDE Features** implemented
- ✅ **Diagnostics**: Real-time error detection
- ✅ **Completion**: Smart code completion with documentation
- ✅ **Go to Definition**: Navigate to symbol definitions
- ✅ **Find References**: Find all usages of symbols
- ✅ **Hover**: Type information and documentation
- ✅ **Signature Help**: Parameter hints while typing
- ✅ **Symbol Navigation**: Document and workspace symbols
- ✅ **Rename**: Safe symbol renaming across files
- ✅ **Code Actions**: Quick fixes and refactorings
- ✅ **Formatting**: Document and on-type formatting
- ✅ **Semantic Tokens**: Syntax highlighting
- ✅ **Type Hierarchy**: Navigate inheritance chains
- ✅ **Call Hierarchy**: Trace function calls
- ✅ **Document Links**: MetaCPAN and local file links

### Refactoring Features
- Extract variable/subroutine
- Convert loops (for/while/foreach)
- Add error checking
- Organize imports
- Inline variables

### Performance
- <50ms response times for all operations
- Handles large codebases efficiently
- Smart caching for improved performance

## 🧪 Testing

### Basic Test
```bash
perl-lsp --version
# Should output: perl-lsp 0.1.0
# Perl Language Server using perl-parser v3
```

### LSP Protocol Test
```bash
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio
# Should return JSON response with capabilities
```

## 🐛 Troubleshooting

### Binary not found
```bash
# Check if installed
which perl-lsp

# Check if in PATH
echo $PATH | tr ':' '\n' | grep -E '(local/bin|usr/local/bin)'

# Add to PATH if needed
export PATH="/usr/local/bin:$PATH"
```

### VS Code not detecting LSP
1. Check VS Code settings for `perl-lsp.serverPath`
2. Restart VS Code after configuration
3. Check Output panel > Perl LSP for errors

### Permission denied
```bash
chmod +x /usr/local/bin/perl-lsp
```

## 📊 Package Info

| Platform | File | Size | SHA256 |
|----------|------|------|--------|
| Linux x86_64 | perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu.tar.gz | 1.5MB | 49c6621f2ede2822f202dfaafa52c9dbe8351af36368f53e3d85b20094efa1f1 |

## 🚦 Next Steps

1. **Test thoroughly** with your Perl codebases
2. **Report issues** internally before public release
3. **Gather feedback** from team members
4. **Plan public release** when ready

## 📝 Notes

- This is an internal release candidate (RC) for testing
- GitHub Actions are disabled for the private repository
- Public release will enable automated builds for all platforms
- For now, use the local build script for additional platforms

## 🔗 Links

- Repository: (internal - private)
- Public release planned: v0.8.3 (after internal validation)