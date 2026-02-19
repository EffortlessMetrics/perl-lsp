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

### Completion Not Working

**Problem**: No completions appear when typing.

**Solutions**:

1. Ensure you're in a valid completion context (after `$`, `@`, `%`, or mid-identifier)

2. Check the file is recognized as Perl in your editor

3. Try manually triggering completion:
   - VS Code: `Ctrl+Space`
   - Neovim: `<C-x><C-o>`
   - Emacs: `M-TAB` or `C-M-i`

4. Verify completion cap hasn't been reached:
   ```json
   {
     "perl": {
       "limits": {
         "completionCap": 200
       }
     }
   }
   ```

### Go-to-Definition Not Working

**Problem**: "Go to Definition" doesn't find the symbol.

**Solutions**:

1. Ensure the definition is in an indexed file:
   - File must be in workspace or `includePaths`
   - File count must be under `maxIndexedFiles` limit

2. Check include paths are configured:
   ```json
   {
     "perl": {
       "workspace": {
         "includePaths": ["lib", ".", "local/lib/perl5"]
       }
     }
   }
   ```

3. For CPAN modules, enable system @INC (if safe):
   ```json
   {
     "perl": {
       "workspace": {
         "useSystemInc": true
       }
     }
   }
   ```

4. Verify the symbol is actually defined (not just imported)

### References Returning Incomplete Results

**Problem**: "Find References" doesn't show all occurrences.

**Solutions**:

1. Check the references cap:
   ```json
   {
     "perl": {
       "limits": {
         "referencesCap": 1000
       }
     }
   }
   ```

2. Ensure all relevant files are indexed (check `maxIndexedFiles`)

3. Wait for workspace indexing to complete (check progress notification)

4. Check the reference search deadline:
   ```json
   {
     "perl": {
       "limits": {
         "referenceSearchDeadlineMs": 5000
       }
     }
   }
   ```

### Formatting Not Working

**Problem**: Document formatting doesn't change the file.

**Solutions**:

1. Verify Perl::Tidy is installed:
   ```bash
   perl -e 'use Perl::Tidy;'
   ```

2. Check for `.perltidyrc` in your project or home directory

3. Verify formatting is enabled in editor settings

4. Check for errors in the LSP log - formatting errors are often reported there

5. Try manual formatting via command palette to see error messages

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

3. File an issue at: https://github.com/EffortlessMetrics/perl-lsp/issues

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

### VS Code: Extension Not Activating

**Problem**: The perl-lsp extension doesn't activate on Perl files.

**Solutions**:

1. Check file association in VS Code bottom status bar (should say "Perl")

2. Manually set language mode: `Ctrl+K M` then select "Perl"

3. Verify extension is installed and enabled:
   ```bash
   code --list-extensions | grep perl
   ```

4. Check VS Code Output panel for errors (View > Output > select "Perl Language Server")

### Neovim: LSP Not Attaching

**Problem**: `:LspInfo` shows no client attached.

**Solutions**:

1. Verify filetype is recognized:
   ```vim
   :set filetype?
   " Should output: filetype=perl
   ```

2. Check lspconfig is loaded:
   ```vim
   :lua print(vim.inspect(require('lspconfig').perl_lsp))
   ```

3. Manually start the client:
   ```vim
   :LspStart perl_lsp
   ```

4. Check `:LspLog` for errors

### Emacs: eglot Fails to Connect

**Problem**: eglot reports connection failure.

**Solutions**:

1. Check the `*eglot stderr*` buffer for errors

2. Verify the command works in shell:
   ```bash
   perl-lsp --stdio
   ```

3. Try lsp-mode as an alternative:
   ```elisp
   (require 'lsp-mode)
   (add-hook 'perl-mode-hook #'lsp)
   ```

4. Check `*lsp-log*` buffer for detailed errors

## Getting Help

1. Check existing issues: https://github.com/EffortlessMetrics/perl-lsp/issues

2. Enable debug logging and include logs in bug reports:
   ```bash
   RUST_LOG=perl_lsp=debug,perl_parser=debug perl-lsp --stdio 2>debug.log
   ```

3. Include:
   - perl-lsp version (`perl-lsp --version`)
   - Rust version (`rustc --version`)
   - OS and editor
   - Minimal code reproduction

## See Also

- [FAQ.md](FAQ.md) - Frequently asked questions
- [GETTING_STARTED.md](GETTING_STARTED.md) - Installation and setup guide
- [EDITOR_SETUP.md](EDITOR_SETUP.md) - Detailed editor configurations
- [CONFIG.md](CONFIG.md) - All configuration options
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) - Current parser limitations
