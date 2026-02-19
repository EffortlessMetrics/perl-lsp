# perl-regex

Regex validation and safety analysis for Perl regular expression patterns.

## Features

- **Nested quantifier detection** -- heuristic detection of patterns like `(a+)+` that risk catastrophic backtracking.
- **Embedded code detection** -- identifies `(?{...})` and `(??{...})` constructs that execute arbitrary Perl code.
- **Complexity checking** -- enforces limits on lookbehind nesting depth, branch reset nesting, Unicode property count, and branch count.
- **Offset-aware errors** -- all diagnostics carry the source offset for IDE integration.

## Public API

| Type | Purpose |
|------|---------|
| `RegexValidator` | Configurable validator with safety limits (nesting depth, Unicode properties) |
| `RegexError` | Error type with source offset for syntax/security issues |

## Workspace Role

Tier 1 leaf crate in the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/perl-lsp) workspace. Used by parse-time validation logic to flag risky regex patterns for LSP diagnostics.

## License

MIT OR Apache-2.0
