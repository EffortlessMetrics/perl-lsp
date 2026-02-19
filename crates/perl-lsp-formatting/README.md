# perl-lsp-formatting

LSP formatting provider for Perl with perltidy integration.

## Overview

This crate provides document and range formatting for Perl source code by
invoking `perltidy` as a subprocess. It is a Tier 2 workspace crate consumed
by the `perl-lsp` server to handle `textDocument/formatting` and
`textDocument/rangeFormatting` requests.

## Public API

- `FormattingProvider<R>` -- generic over a `SubprocessRuntime` from `perl-lsp-tooling`; runs perltidy and returns edits.
- `FormattingOptions` -- tab size, spaces-vs-tabs, trailing whitespace, final newline settings.
- `FormattingError` -- error variants for missing perltidy, execution failures, and I/O errors.
- `FormattedDocument`, `FormatTextEdit`, `FormatRange`, `FormatPosition` -- result and edit types.

## Prerequisites

Requires `perltidy` on `PATH` (or a custom path via `FormattingProvider::with_perltidy_path`).
See `FormattingError::PerltidyNotFound` for per-platform install instructions.

## Workspace

Part of [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs).

## License

MIT OR Apache-2.0
