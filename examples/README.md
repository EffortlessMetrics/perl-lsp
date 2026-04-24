# Perl Language Server -- Examples

This directory contains curated examples that demonstrate how to use the Perl
parser library and the Language Server Protocol (LSP) integration, together
with Perl files that exercise every major LSP feature.

---

## Rust Examples

### `parse_file.rs`

Parses a Perl file and prints the AST in S-expression format.

```bash
cargo run --example parse_file -- examples/perl/simple.pl
```

### `lsp_client.rs`

Demonstrates programmatic interaction with the Perl Language Server over stdio
(initialize, open document, request symbols, shutdown).

```bash
# Build the LSP server binary first
cargo build -p perllsp --release

# Then run the example (requires perl-lsp in PATH)
cargo run --example lsp_client
```

---

## Perl Showcase Files (`perl/`)

Each file is a self-contained, runnable Perl program annotated with which LSP
features it exercises.  Open any file in an editor with `perl-lsp` configured
and interact with it to see the features in action.

### `perl/simple.pl` -- Core syntax

Covers variable declarations, subroutines, control flow, loops, and hash
iteration.  Good starting point for confirming the parser handles everyday Perl.

**LSP features:** completion, hover on built-ins, go-to-def for subs.

### `perl/complex.pl` -- Advanced parser edge cases

Regex with non-slash delimiters, Unicode identifiers, complex dereferencing,
here-documents, given/when, subroutine signatures, and postfix dereferencing.

**LSP features:** hover on regex operators, diagnostics on experimental features.

### `perl/oop.pl` -- Object-oriented Perl (Moose / Moo)

Moose class with attributes and methods, subclass with `extends`, Moo role with
`requires`, and polymorphic method dispatch.

**LSP features:**
- **hover** -- hover over a method name to see its signature and docs
- **go-to-def** -- jump from `$dog->fetch()` to `Dog::fetch`
- **rename** -- rename `speak` and have all call sites updated
- **completion** -- type `$self->` to see available methods

### `perl/regex.pl` -- Regular expressions

Match/capture, named captures, substitution with `/e`, `/x` verbose patterns,
lookahead/lookbehind, precompiled `qr//`, global match, and `tr///`.

**LSP features:**
- **hover** -- hover over a regex to see an inline pattern explainer
- **diagnostics** -- warnings for known anti-patterns

### `perl/unicode.pl` -- Unicode support

UTF-8 source files, Unicode string literals, `\N{...}` named characters,
Unicode-aware regex (`\p{L}`), and `Encode` round-trips.

**LSP features:**
- **hover** -- hover over `\N{SNOWMAN}` to see the codepoint
- **diagnostics** -- warn when non-ASCII appears without `use utf8`

### `perl/modules.pl` -- Module system and cross-file navigation

`use` and `use lib`, multiple inline packages, `Exporter`, nested namespaces
(`Config::Database`), `use constant`, and dynamic `require`.

**LSP features:**
- **go-to-def** -- jump from `use List::Util` to the module source
- **completion** -- imported symbols appear in the completion list
- **hover** -- show function signatures from module POD
- **rename** -- rename an exported sub across all callers

### `perl/modern.pl` -- Modern Perl (5.20 to 5.38)

Subroutine signatures, `try`/`catch`, `state` variables, postfix
dereferencing, and the experimental `class`/`method` syntax (Corinna, 5.38).

**LSP features:**
- **diagnostics** -- catch undefined variables inside signatures
- **hover** -- show parameter types when hovering over sub names
- **completion** -- parameter names appear in completions
- **go-to-def** -- jump into `class` definitions from instantiation sites

---

## Batch Parse All Perl Examples

```bash
for file in examples/perl/*.pl; do
    echo "=== $file ==="
    cargo run --example parse_file -- "$file" 2>&1 | head -5
done
```
