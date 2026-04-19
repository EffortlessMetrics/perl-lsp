# ADR: Native Rust `PerlLanguage` Descriptor for `tree-sitter-perl-rs`

## Status
Accepted

## Context
The `tree-sitter-perl-rs` crate provides a Rust-native Perl parser with tree-sitter-style ergonomics. The tree-sitter ecosystem conventionally expects every grammar crate to expose a `language()` function returning a language descriptor. Without it, `tree-sitter-perl-rs` cannot serve as a drop-in component in tooling that follows this convention.

`tree-sitter-perl-rs` intentionally does not depend on the `tree-sitter` crate or any C toolchain — it is a pure Rust facade over the native v3 parser. The `tree_sitter::Language` type is an opaque C FFI wrapper (`TSLanguage*`) that cannot be constructed without the tree-sitter-cli generated C code.

## Decision
We implement **Option A**: a Rust-native `PerlLanguage` descriptor struct that provides an informational API backed by `perl_ast::NodeKind::ALL_KIND_NAMES`. This is not a `tree_sitter::Language` substitute — it is a separate type that serves Rust-native tooling needs.

### API Shape

```rust
pub struct PerlLanguage {
    kind_names: &'static [&'static str],
}

impl PerlLanguage {
    pub fn node_kind_count(&self) -> usize;
    pub fn node_kind_names(&self) -> &[&'static str];
    pub fn node_kind_is_named(&self, kind: &str) -> bool;
}

pub fn language() -> PerlLanguage;
pub static LANGUAGE: PerlLanguage;
```

### Implementation Notes
- `LANGUAGE` is constructed from `perl_ast::NodeKind::ALL_KIND_NAMES`
- `LANGUAGE` is `Sync` because `&'static [&'static str]` is `Sync`
- The `language()` function follows the tree-sitter ecosystem convention for discoverability
- Doc comments explicitly state this is NOT `tree_sitter::Language` and direct users to `tree-sitter-perl-c` for drop-in compatibility

## Consequences

### Positive
- `tree-sitter-perl-rs` remains C-free; no `tree-sitter` crate dependency introduced
- Rust tooling can query node kind metadata without C FFI
- `language()` convention enables discoverability in tree-sitter-aware tooling
- Purely additive change; no existing APIs modified
- `LANGUAGE` static automatically tracks changes to `ALL_KIND_NAMES` as grammar evolves

### Negative
- `PerlLanguage` cannot be passed to `tree_sitter::Parser::set_language`
- Users needing `tree_sitter::Language` must use `tree-sitter-perl-c`
- `node_kind_is_named()` semantics ("exists in ALL_KIND_NAMES") conflates existence with named/anonymous status — acceptable for v3 internal kinds

## Alternatives Considered

### Option B: Re-export from `tree-sitter-perl-c`
Have `tree-sitter-perl-rs` depend on `tree-sitter-perl-c` and re-export its `language()`. Rejected: introduces C toolchain dependency into the Rust-native crate, defeating its positioning.

### Option C: Implement `tree_sitter::Language` natively
Attempt to construct `tree_sitter::Language` from pure Rust. Rejected: `tree_sitter::Language` wraps a C pointer (`TSLanguage*`) from tree-sitter-cli's generated code; it is opaque and cannot be constructed without linking the C grammar.

## References
- Issue: `feat(tree-sitter-perl-rs): add PerlLanguage descriptor, language() function, and LANGUAGE constant (Phase 2, gap 6/6)`
- ROADMAP.md lines 31-35: Phase 2 item for "Language constant"
- CLAUDE.md line 81: backlog item for Language constant
- `crates/tree-sitter-perl-c/src/lib.rs:60-67`: reference `language() -> tree_sitter::Language` via C FFI