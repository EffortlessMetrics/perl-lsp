# Product Overview

perl-lsp is a native Rust implementation of a Language Server Protocol (LSP) server and Debug Adapter Protocol (DAP) server for Perl 5. It provides IDE features like completions, diagnostics, hover, go-to-definition, find references, rename, formatting, semantic tokens, inlay hints, code actions, code lens, and workspace symbols — all without requiring a Perl runtime for IDE features.

The project also includes a native recursive-descent Perl parser, a context-aware tokenizer, semantic analysis, and cross-file workspace indexing, all usable as standalone library crates.

Key capabilities:
- Complete LSP surface (88 LSP + 24 DAP + 7 extension capabilities)
- Native debug adapter with breakpoints, stepping, stack frames, variable inspection
- Semantic analysis with symbol resolution, scope tracking, Moose/Moo support
- Refactoring: extract variable, extract subroutine, workspace-scoped rename, subroutine inlining
- Diagnostics: dead code, strict/warnings, perlcritic integration
- Cross-platform: Windows, macOS, Linux

Current version: v0.13.0-rc1 (public alpha). Dual-licensed MIT / Apache-2.0.

Editor support: VS Code (primary, with bundled extension), Neovim, Emacs, Zed, Helix.
