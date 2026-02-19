# perl-lsp-providers

LSP provider aggregation and tooling integrations for Perl.

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace (Tier 4).

## Overview

This crate is the central aggregation point for all LSP feature providers in the Perl LSP ecosystem. It re-exports individual microcrates (completion, diagnostics, formatting, navigation, rename, semantic tokens, inlay hints, code actions) through a unified API and provides IDE integration shims and backward-compatible legacy import paths.

## Public API

Top-level re-export modules (preferred for new code):

- `completion` -- from `perl-lsp-completion`
- `diagnostics` -- from `perl-lsp-diagnostics`
- `formatting` -- from `perl-lsp-formatting`
- `navigation` -- from `perl-lsp-navigation`
- `rename` -- from `perl-lsp-rename`
- `semantic_tokens` -- from `perl-lsp-semantic-tokens`
- `inlay_hints` -- from `perl-lsp-inlay-hints`
- `code_actions` -- from `perl-lsp-code-actions`
- `tooling` -- from `perl-lsp-tooling` (perltidy, perlcritic)
- `ide` -- LSP/DAP runtime compatibility shims

Core parser types (`Node`, `NodeKind`, `SourceLocation`, `Parser`, `ast`, `position`) are re-exported from `perl-parser-core`.

## Features

- `lsp-compat` (default) -- enables LSP protocol type compatibility via `lsp-types`

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0) or [MIT license](http://opensource.org/licenses/MIT) at your option.
