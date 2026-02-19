# perl-dap

Debug Adapter Protocol (DAP) server for Perl, enabling debugging in VS Code, Neovim, Emacs, and other DAP-compatible editors.

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace.

## Features

- **Native adapter** (default): drives `perl -d` directly over stdio or TCP socket
- **Bridge adapter** (library): proxies DAP messages to Perl::LanguageServer
- **Breakpoint management** with AST-based validation via `perl-dap-breakpoint`
- **Variable inspection** and **safe expression evaluation** via `perl-dap-variables` / `perl-dap-eval`
- **Stack trace parsing** via `perl-dap-stack`
- **Security hardening**: path traversal prevention, expression sanitization, resource limits
- **Cross-platform**: Linux, macOS, Windows, and WSL path translation

## Usage

```bash
cargo install --path crates/perl-dap
perl-dap --stdio            # Native adapter on stdio (default)
perl-dap --socket --port 13603  # Native adapter on TCP
perl-dap --bridge           # Bridge mode (requires Perl::LanguageServer)
```

## License

MIT OR Apache-2.0
