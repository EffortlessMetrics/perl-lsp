# FAQ

## How do I install perl-lsp?
- crates.io: `cargo install perl-lsp`
- Releases: https://github.com/EffortlessMetrics/perl-lsp/releases
- Installer (Linux/macOS, best-effort):  
  `curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash`
- From source (development default):
  `cargo install --path crates/perl-lsp`

## Does the installer install perl-dap?
No. The installer installs `perl-lsp`. Build/run `perl-dap` from source when needed.

## Where is feature coverage tracked?
`features.toml` is canonical. Computed metrics live in `docs/CURRENT_STATUS.md`.

## Where do I report bugs?
Open an issue with a minimal repro (smallest Perl snippet + expected vs actual).

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
