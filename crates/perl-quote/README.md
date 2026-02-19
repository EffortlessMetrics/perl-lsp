# perl-quote

Perl quote-like operator parsing helpers for extracting patterns, bodies, and modifiers
from regex, substitution, and transliteration operators.

## Public API

- `extract_regex_parts(text) -> (pattern, body, modifiers)` -- parse `qr//`, `m//`, and bare `//` regex tokens
- `extract_substitution_parts(text) -> (pattern, replacement, modifiers)` -- parse `s///` with any delimiter style (lenient)
- `extract_substitution_parts_strict(text) -> Result<(pattern, replacement, modifiers), SubstitutionError>` -- strict variant that rejects invalid modifiers
- `extract_transliteration_parts(text) -> (search, replacement, modifiers)` -- parse `tr///` and `y///`
- `validate_substitution_modifiers(text) -> Result<String, char>` -- validate modifier characters
- `SubstitutionError` -- error enum for strict parsing failures

Handles paired (`{}`, `[]`, `()`, `<>`) and non-paired delimiters, nested delimiter
balancing, and escape sequences.

## Workspace Role

Tier 1 leaf crate with zero dependencies. Used by `perl-parser-core` for
quote operator content extraction during parsing.

Part of the [tree-sitter-perl-rs](https://github.com/EffortlessMetrics/tree-sitter-perl-rs) workspace.

## License

MIT OR Apache-2.0
