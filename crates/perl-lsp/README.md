# perl-lsp

[![Crates.io](https://img.shields.io/crates/v/perl-lsp.svg)](https://crates.io/crates/perl-lsp)
[![Documentation](https://docs.rs/perl-lsp/badge.svg)](https://docs.rs/perl-lsp)

Use this crate when you need the actual Perl language server entry point, not
just parser or provider pieces.

## When to use this crate

Use `perl-lsp` when you want to run or embed the real language server:

- run the `perl-lsp` binary behind an editor such as VS Code, Neovim, Emacs, or Helix
- expose Perl LSP features over stdio or TCP
- embed the server entry point from Rust instead of shelling out to a binary

If you only need a parser, tokenizer, or a single feature provider, prefer the
smaller workspace crates such as `perl-parser`, `perl-lexer`, or the
`perl-lsp-*` provider crates.

## Installation

```bash
cargo install perl-lsp
```

## Quick start

```bash
perl-lsp --stdio
perl-lsp --health
```

## Usage

```bash
perl-lsp --stdio          # stdio mode (default, for editor integration)
perl-lsp --socket --port 9257  # TCP socket mode
perl-lsp --health         # health check
perl-lsp --version        # version info
```

## Embedding from Rust

The `perl_lsp` library re-exports `LspServer`, `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, and a convenience `run_stdio()` entry point for embedding.

## Workspace role

This is the executable entry point in the
[`perl-lsp`](https://github.com/EffortlessMetrics/perl-lsp) workspace. It
delegates parsing to `perl-parser` and dispatches feature work through focused
provider crates such as `perl-lsp-completion`, `perl-lsp-navigation`, and
`perl-lsp-diagnostics`.

## License

MIT OR Apache-2.0
