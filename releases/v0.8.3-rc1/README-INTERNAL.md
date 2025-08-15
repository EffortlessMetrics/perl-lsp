# Perl LSP v0.8.3-rc1 Internal Release

## 🚀 Quick Start

### Installation (Linux/macOS)

```bash
# Extract the package
tar -xzf perl-lsp-v0.8.3-rc1-x86_64-unknown-linux-gnu.tar.gz

# Install to system (requires sudo)
sudo cp perl-lsp-v0.8.3-rc1-*/perl-lsp /usr/local/bin/

# Or install to user directory (no sudo needed)
mkdir -p ~/.local/bin
cp perl-lsp-v0.8.3-rc1-*/perl-lsp ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"

# Verify installation
perl-lsp --version
```

### VS Code Integration

1. Install the binary as shown above
2. Configure VS Code settings:

```json
{
  "perl.lsp.path": "/usr/local/bin/perl-lsp",
  "perl.lsp.enabled": true
}
```

3. Open any `.pl` file and the LSP will start automatically

### Testing the LSP

```bash
# Test with stdio
printf 'Content-Length: 59\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio

# Test with a Perl file
perl-lsp --check your-script.pl
```

## 📦 Package Contents

- `perl-lsp` - The Language Server Protocol binary
- `README.md` - Main project documentation  
- `LICENSE` - MIT License

## 🔧 Features (v0.8.3-rc1)

### Core LSP Features
- ✅ Real-time diagnostics
- ✅ Auto-completion
- ✅ Go to definition
- ✅ Find references
- ✅ Hover documentation
- ✅ Signature help
- ✅ Document/workspace symbols
- ✅ Rename refactoring

### Advanced Features
- ✅ Code actions and quick fixes
- ✅ Extract variable/subroutine
- ✅ Type hierarchy navigation
- ✅ Call hierarchy
- ✅ Document links (MetaCPAN + local)
- ✅ Selection ranges
- ✅ On-type formatting
- ✅ Semantic tokens
- ✅ CodeLens
- ✅ Inlay hints
- ✅ Folding ranges

### Perl Support
- ✅ 100% Perl 5 syntax coverage
- ✅ Modern Perl features (5.38+)
- ✅ Unicode identifiers
- ✅ All regex delimiters
- ✅ Heredocs (all variants)
- ✅ 150+ built-in functions

## 🐛 Known Issues

- This is a release candidate (RC) for internal testing
- GitHub Actions are disabled (internal repo)
- ARM64 Linux packages not included in this build

## 📊 Performance

- Simple files: ~1-2ms parsing
- Medium files (1000 lines): ~50-150ms
- Large files (10000+ lines): ~500ms-1s
- Memory usage: ~20-50MB typical

## 🔍 Troubleshooting

### LSP not starting
```bash
# Check binary is executable
ls -la /usr/local/bin/perl-lsp
# Should show: -rwxr-xr-x

# Test manually
perl-lsp --version
# Should show: perl-lsp 0.1.0

# Enable debug logging
perl-lsp --stdio --log 2>lsp.log
```

### VS Code not detecting LSP
1. Check Output panel → "Perl Language Server"
2. Ensure settings point to correct binary path
3. Reload VS Code window (Cmd/Ctrl+Shift+P → "Reload Window")

## 📝 Feedback

Internal testing feedback:
- Report issues in internal tracker
- Test with your production Perl codebases
- Note any missing features or false positives

## 🚦 Release Status

- **v0.8.3-rc1**: Internal testing phase
- **v0.8.3**: Planned public release (pending testing)
- **v1.0.0**: GA release (Q1 2025)

---

*This is an internal release for testing purposes only.*