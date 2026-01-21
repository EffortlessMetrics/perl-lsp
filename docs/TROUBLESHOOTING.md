# Troubleshooting Guide

Common issues and their solutions when using perl-lsp.

## Quick Diagnostics

```bash
# Check installation
which perl-lsp && perl-lsp --version

# Health check
perl-lsp --health

# Test JSON-RPC communication
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
```

## Build Issues

### Compilation Fails

**Problem**: `cargo build` fails with errors.

**Solutions**:

1. Ensure you have Rust 1.89+ (MSRV):
   ```bash
   rustup update stable
   rustc --version  # Should be >= 1.89
   ```

2. Clean and rebuild:
   ```bash
   cargo clean
   cargo build -p perl-lsp --release
   ```

3. If using Nix:
   ```bash
   nix develop -c cargo build -p perl-lsp --release
   ```

### Missing Dependencies

**Problem**: Build complains about missing system dependencies.

**Solution**: perl-lsp is pure Rust and should not require system dependencies. If you see C compiler or libclang errors, you may be building optional crates. Use:

```bash
cargo build -p perl-lsp --release
```

Not `cargo build --workspace` which includes optional native crates.

## Installation Issues

### Binary Not Found

**Problem**: `perl-lsp: command not found` after installation.

**Solutions**:

1. Check Cargo's bin directory is in PATH:
   ```bash
   echo $PATH | tr ':' '\n' | grep cargo
   # Should include: ~/.cargo/bin
   ```

2. Add to PATH if missing:
   ```bash
   # Add to ~/.bashrc or ~/.zshrc
   export PATH="$HOME/.cargo/bin:$PATH"
   ```

3. Verify installation location:
   ```bash
   ls -la ~/.cargo/bin/perl-lsp
   ```

### Permission Denied

**Problem**: Cannot execute perl-lsp binary.

**Solution**:
```bash
chmod +x ~/.cargo/bin/perl-lsp
```

## Runtime Issues

### Server Crashes on Startup

**Problem**: perl-lsp exits immediately or crashes.

**Solutions**:

1. Run with debug logging:
   ```bash
   RUST_LOG=perl_lsp=debug perl-lsp --stdio 2>debug.log
   ```

2. Check for conflicting processes:
   ```bash
   ps aux | grep perl-lsp
   ```

3. Verify the binary is not corrupted:
   ```bash
   cargo install --path crates/perl-lsp --force
   ```

### High Memory Usage

**Problem**: perl-lsp uses excessive memory on large projects.

**Solutions**:

1. Limit indexed files:
   ```json
   {
     "perl": {
       "limits": {
         "maxIndexedFiles": 1000
       }
     }
   }
   ```

2. Exclude directories via workspace settings:
   ```json
   {
     "perl": {
       "workspace": {
         "excludePaths": ["node_modules", "vendor", ".git"]
       }
     }
   }
   ```

### Slow Performance

**Problem**: LSP responses are slow.

**Solutions**:

1. Disable unused features:
   ```json
   {
     "perl": {
       "enableSemanticTokens": false,
       "enableInlayHints": false
     }
   }
   ```

2. Reduce workspace scope - see [EDITOR_SETUP.md](EDITOR_SETUP.md#slow-performance)

### No Diagnostics Appearing

**Problem**: Syntax errors are not highlighted.

**Solutions**:

1. Ensure file has Perl extension: `.pl`, `.pm`, or `.t`

2. Check editor recognizes file as Perl (language mode)

3. Verify diagnostics are enabled in settings

4. Check LSP logs for errors - see [EDITOR_SETUP.md](EDITOR_SETUP.md#no-diagnostics)

## Parser Issues

### Incorrect Syntax Highlighting

**Problem**: Code is parsed incorrectly or shows false errors.

**Solutions**:

1. Check if the syntax is a known limitation - see [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md)

2. Report unhandled syntax:
   ```bash
   # Create minimal reproduction
   cat > test.pl << 'EOF'
   # Your problematic code here
   EOF

   # Test parsing
   perl-lsp --parse test.pl
   ```

3. File an issue at: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues

### Heredoc Issues

**Problem**: Heredocs not parsed correctly.

**Solution**: Ensure heredoc delimiters are on their own lines:

```perl
# Works
my $text = <<'END';
content here
END

# May not work
my $text = <<'END'; print "after heredoc";
content
END
```

## DAP (Debug Adapter) Issues

**Note**: DAP support is experimental. Current limitations:

- Launch mode only (attach pending)
- Variables/evaluate show placeholders
- BridgeAdapter library available for advanced use

### Debugger Not Starting

**Problem**: Debug session fails to start.

**Solutions**:

1. Ensure perl-dap is installed:
   ```bash
   cargo install --path crates/perl-dap
   ```

2. Verify Perl::LanguageServer is available (for bridge mode):
   ```bash
   perl -e 'use Perl::LanguageServer;'
   ```

## Editor-Specific Issues

For detailed editor configuration and troubleshooting:

- [VS Code setup](EDITOR_SETUP.md#vs-code)
- [Neovim setup](EDITOR_SETUP.md#neovim)
- [Emacs setup](EDITOR_SETUP.md#emacs)
- [Helix setup](EDITOR_SETUP.md#helix)
- [General troubleshooting](EDITOR_SETUP.md#troubleshooting)

## Getting Help

1. Check existing issues: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues

2. Enable debug logging and include logs in bug reports:
   ```bash
   RUST_LOG=perl_lsp=debug,perl_parser=debug perl-lsp --stdio 2>debug.log
   ```

3. Include:
   - perl-lsp version (`perl-lsp --version`)
   - Rust version (`rustc --version`)
   - OS and editor
   - Minimal code reproduction
