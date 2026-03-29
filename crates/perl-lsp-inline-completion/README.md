# perl-lsp-inline-completion

[![Crates.io](https://img.shields.io/crates/v/perl-lsp-inline-completion.svg)](https://crates.io/crates/perl-lsp-inline-completion)
[![Documentation](https://docs.rs/perl-lsp-inline-completion/badge.svg)](https://docs.rs/perl-lsp-inline-completion)

Deterministic inline-completion support for Perl editors and language servers.

## When to use this crate

Use `perl-lsp-inline-completion` when you want ghost-text suggestions driven by
local code context instead of a remote model. The provider extracts nearby
variables, imports, package context, and simple syntactic cues to produce
predictable inline completions.

## Quick example

```rust,ignore
use perl_lsp_inline_completion::InlineCompletionProvider;

let provider = InlineCompletionProvider::new();
let list = provider.get_inline_completions("my $name = 'A';\nprint $na", 1, 9);
assert!(!list.items.is_empty());
```

## Public API

- `InlineCompletionProvider`: main provider
- `PreparedInlineCompletionContext`: extracted local code context
- `InlineCompletionItem` and `InlineCompletionList`: LSP 3.18 preview types

## License

MIT OR Apache-2.0
