# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-quote` is a **Tier 1 leaf crate** that extracts patterns, bodies, and modifiers from Perl quote-like operator token text.

**Purpose**: Parse regex (`qr//`, `m//`), substitution (`s///`), and transliteration (`tr///`, `y///`) operators, handling all Perl delimiter styles and modifier validation.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-quote            # Build this crate
cargo test -p perl-quote             # Run tests
cargo clippy -p perl-quote           # Lint
cargo doc -p perl-quote --open       # View documentation
```

## Architecture

### Dependencies

**None** -- pure Rust, zero external dependencies. Only imports `std::borrow::Cow`.

### Key Public API

| Function / Type | Purpose |
|----------------|---------|
| `extract_regex_parts(text)` | Parse `qr//`, `m//`, bare `//` into (pattern, body, modifiers) |
| `extract_substitution_parts(text)` | Lenient `s///` parsing -- silently filters invalid modifiers |
| `extract_substitution_parts_strict(text)` | Strict `s///` parsing -- returns `SubstitutionError` on invalid input |
| `extract_transliteration_parts(text)` | Parse `tr///` and `y///` into (search, replacement, modifiers) |
| `validate_substitution_modifiers(text)` | Validate modifier string, returns `Err(char)` on first invalid char |
| `SubstitutionError` | Error enum: `InvalidModifier`, `MissingDelimiter`, `MissingPattern`, `MissingReplacement`, `MissingClosingDelimiter` |

### Internal Helpers

- `get_closing_delimiter(open)` -- maps `{`->`}`, `[`->`]`, `(`->`)`, `<`->`>`, or same char
- `extract_delimited_content(text, open, close)` -- balanced delimiter extraction with escape handling
- `extract_delimited_content_strict(...)` -- same but tracks whether closing delimiter was found
- `extract_unpaired_body(text, closing)` -- extract body for non-paired delimiters
- `extract_substitution_pattern_with_replacement_hint(...)` -- paired-delimiter substitution with lookahead for replacement section
- `split_unclosed_substitution_pattern(...)` / `split_on_last_paired_delimiter(...)` -- fallback parsing for edge cases

### Delimiter Handling

Supports paired delimiters (`{}`, `[]`, `()`, `<>`) with nested depth counting, and non-paired delimiters (any non-alphanumeric character like `/`, `!`, `#`). Backslash escapes are respected in all modes.

### Valid Modifiers

- **Substitution**: `g`, `i`, `m`, `s`, `x`, `o`, `e`, `r`, `a`, `d`, `l`, `u`, `n`, `p`, `c`
- **Transliteration**: `c`, `d`, `s`, `r`

## Usage

```rust
use perl_quote::{extract_regex_parts, extract_substitution_parts, extract_transliteration_parts};

let (pattern, body, mods) = extract_regex_parts("qr{foo|bar}ix");
assert_eq!(body, "foo|bar");
assert_eq!(mods, "ix");

let (pat, repl, mods) = extract_substitution_parts("s/foo/bar/gi");
assert_eq!(pat, "foo");
assert_eq!(repl, "bar");
assert_eq!(mods, "gi");

let (search, repl, mods) = extract_transliteration_parts("tr/a-z/A-Z/");
assert_eq!(search, "a-z");
assert_eq!(repl, "A-Z");
```

## Important Notes

- This crate operates on raw token text (the `s`, `tr`, `qr` prefix is expected in the input)
- Paired delimiters support nesting: `s{a{b}c}{replacement}`
- For substitution with paired delimiters, pattern and replacement may use different delimiters: `s[pattern]{replacement}`
- Heredoc handling is in the separate `perl-heredoc` crate
- Only `perl-parser-core` depends on this crate in the workspace
