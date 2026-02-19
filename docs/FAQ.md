# Frequently Asked Questions

Common questions about perl-lsp, organized by topic.

## Installation & Setup

### What are the system requirements?

- **Rust 1.92+** for building from source
- No runtime dependencies (perl-lsp is a standalone Rust binary)
- Any operating system: Linux, macOS, Windows

### Do I need Perl installed?

No. perl-lsp parses Perl code natively without requiring a Perl interpreter.

However, some features benefit from having Perl available:
- **Formatting**: Uses Perl::Tidy if installed
- **Test Runner**: Executes tests via `perl` or `prove`
- **Perl::Critic**: Runs perlcritic for style checking

### Where should I install the binary?

Anywhere in your `PATH`. Common locations:

```bash
# User-local (recommended)
~/.cargo/bin/perl-lsp

# System-wide
/usr/local/bin/perl-lsp
```

### How do I update to the latest version?

```bash
# From crates.io
cargo install perl-lsp --force

# From source
git pull && cargo install --path crates/perl-lsp --force
```

---

## Editor Integration

### Which editors are supported?

Any editor with LSP support works. We provide documentation for:
- VS Code (official extension available)
- Neovim (via nvim-lspconfig)
- Emacs (via lsp-mode or eglot)
- Helix
- Sublime Text (via LSP package)

See [EDITOR_SETUP.md](EDITOR_SETUP.md) for detailed configurations.

### Why isn't the language server starting?

Common causes:

1. **Binary not in PATH**:
   ```bash
   which perl-lsp  # Should show the path
   ```

2. **Editor not configured correctly**:
   - Verify the command is `perl-lsp --stdio`
   - Check file type associations for `.pl`, `.pm`, `.t`

3. **Test manually**:
   ```bash
   perl-lsp --health  # Should print "ok <version>"
   ```

### How do I see what the server is doing?

Enable debug logging:

```bash
RUST_LOG=perl_lsp=debug perl-lsp --stdio 2>perl-lsp.log
```

Or check your editor's LSP output panel:
- VS Code: View > Output > select "Perl Language Server"
- Neovim: `:LspLog`
- Emacs: `*lsp-log*` buffer

---

## Features

### What LSP features are supported?

perl-lsp implements 100% of advertised LSP 3.18 features:

| Category | Features |
|----------|----------|
| **Navigation** | Go to Definition, References, Implementation, Type Definition |
| **Editing** | Completion, Signature Help, Hover, Rename |
| **Analysis** | Diagnostics, Document Symbols, Workspace Symbols |
| **Formatting** | Document Formatting, Range Formatting, On-Type Formatting |
| **Advanced** | Code Actions, Code Lens, Inlay Hints, Semantic Tokens |
| **Hierarchy** | Call Hierarchy, Type Hierarchy |

See [features.toml](../features.toml) for the complete capability catalog.

### Does completion work for CPAN modules?

Yes, but with limitations:
- Modules in your configured `includePaths` are indexed
- System `@INC` modules require `useSystemInc: true`
- Module resolution respects standard `use lib` paths

### Does perl-lsp understand Moose/Moo/Mouse?

Basic support is available:
- `has` attribute declarations are recognized as symbols
- Class inheritance via `extends` is tracked
- Role composition via `with` is detected

Full semantic understanding of Moose meta-object protocol is not implemented.

### Can it refactor my code?

Yes. Available refactorings via Code Actions:
- Extract Variable
- Extract Subroutine
- Convert Loop Styles (C-style to foreach)
- Add Error Checking
- Convert to Postfix
- Organize Imports
- Add Missing Pragmas

Trigger code actions with your editor's quick-fix command.

### Does it support heredocs?

Yes. Heredocs are fully parsed including:
- `<<EOF`, `<<'EOF'`, `<<"EOF"` forms
- Indented heredocs (`<<~EOF`)
- Multiple heredocs on one line

Some edge cases have workarounds - see [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md).

---

## Performance

### How fast is parsing?

- **Initial parse**: 1-150 microseconds for typical files
- **Incremental updates**: ~931 nanoseconds
- **Large files**: Scales linearly at ~7.5 microseconds per KB

### Is it fast enough for large codebases?

Yes. Performance features include:
- Indexed workspace symbols for O(1) lookups
- Configurable result caps (default: 200 symbols, 500 references)
- Deadline enforcement to prevent runaway queries
- AST caching with configurable TTL

For very large projects (50K+ files), tune these settings:

```json
{
  "perl": {
    "limits": {
      "maxIndexedFiles": 50000,
      "workspaceScanDeadlineMs": 120000,
      "astCacheMaxEntries": 200
    }
  }
}
```

### Why is the server using a lot of memory?

Common causes:

1. **Too many files indexed**: Reduce `maxIndexedFiles`
2. **Large AST cache**: Reduce `astCacheMaxEntries`
3. **System @INC included**: Disable `useSystemInc`

```json
{
  "perl": {
    "workspace": { "useSystemInc": false },
    "limits": { "maxIndexedFiles": 5000, "astCacheMaxEntries": 50 }
  }
}
```

---

## Configuration

### Where do I put configuration?

Configuration is provided via your editor's LSP settings:

- **VS Code**: `.vscode/settings.json` or user settings
- **Neovim**: In your `lspconfig.setup()` call
- **Emacs**: In `eglot-workspace-configuration` or `lsp-mode` settings

See [CONFIG.md](CONFIG.md) for all options.

### How do I add custom module search paths?

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5", "my/custom/lib"]
    }
  }
}
```

### How do I disable specific features?

Toggle individual features:

```json
{
  "perl": {
    "inlayHints": { "enabled": false },
    "testRunner": { "enabled": false }
  }
}
```

Or via VS Code extension settings:

```json
{
  "perl-lsp.enableSemanticTokens": false,
  "perl-lsp.enableInlayHints": false
}
```

---

## Debugging

### Does perl-lsp support debugging?

Experimental debugging support is available via **perl-dap** (Debug Adapter Protocol).

Current capabilities:
- Launch mode
- Breakpoints
- Step through code

Not yet implemented:
- Attach mode
- Variable inspection
- Expression evaluation

See [DAP_USER_GUIDE.md](DAP_USER_GUIDE.md) for setup.

### How do I debug the LSP server itself?

1. **Enable logging**:
   ```bash
   RUST_LOG=perl_lsp=debug perl-lsp --stdio 2>debug.log
   ```

2. **Check health**:
   ```bash
   perl-lsp --health
   ```

3. **Test JSON-RPC directly**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | perl-lsp --stdio
   ```

---

## Comparison & Compatibility

### How does perl-lsp compare to Perl::LanguageServer?

| Aspect | perl-lsp | Perl::LanguageServer |
|--------|----------|---------------------|
| **Language** | Rust | Perl |
| **Parser** | Native recursive descent | tree-sitter-perl |
| **Speed** | 1-150 microseconds | Varies |
| **Dependencies** | None (standalone binary) | Requires Perl + CPAN modules |
| **LSP Coverage** | 100% of LSP 3.18 | Subset of LSP features |

### Can I use both LSP servers simultaneously?

Not recommended. Most editors only support one language server per file type. Configure one or the other.

### Is perl-lsp compatible with my Perl version?

perl-lsp parses Perl 5 syntax. It works with code targeting any Perl 5.x version.

Note: It does not require or use a Perl interpreter for parsing - the parser is built into the Rust binary.

---

## Contributing

### How do I report a bug?

Open an issue at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues) with:
- perl-lsp version (`perl-lsp --version`)
- Editor and OS
- Minimal code reproduction
- Debug logs if available

### How do I contribute code?

1. Fork the repository
2. Run the local gate: `nix develop -c just ci-gate`
3. Submit a pull request

See [CONTRIBUTING.md](../CONTRIBUTING.md) for full guidelines.

### Where is the source code?

Repository: [github.com/EffortlessMetrics/perl-lsp](https://github.com/EffortlessMetrics/perl-lsp)

Key directories:
- `crates/perl-lsp/` - LSP server binary
- `crates/perl-parser/` - Parser library
- `crates/perl-dap/` - Debug adapter
- `docs/` - Documentation

---

## See Also

- [GETTING_STARTED.md](GETTING_STARTED.md) - Quick start guide
- [EDITOR_SETUP.md](EDITOR_SETUP.md) - Editor configurations
- [CONFIG.md](CONFIG.md) - Configuration reference
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Problem solutions
- [KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) - Current limitations
