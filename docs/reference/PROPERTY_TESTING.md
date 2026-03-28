# Property-Based Testing

This repository uses [`proptest`](https://docs.rs/proptest) for property-based
testing in places where roundtrips, Unicode edge cases, and generated fixtures
are a better fit than hand-written examples.

## When to Use Property Tests

Use property tests when:

- you are checking a pure transformation or roundtrip
- the input domain is large or awkward to enumerate
- edge cases are more important than one fixed example

Use targeted regression tests when:

- the bug report already provides a concrete failing case
- the behavior depends on workspace layout or server state
- the input domain is small and stable

## Shared Generators

The `perl-test-generators` crate provides reusable strategies for Perl-oriented
inputs:

- `variable()` for sigiled Perl variables
- `module_path()` and `module_path_segments()` for Perl package names
- `unicode_string()` for UTF-8 / UTF-16 and parser edge cases

Example:

```rust
use perl_test_generators::{module_path, unicode_string, variable};
use proptest::prelude::*;

proptest! {
    #[test]
    fn generated_inputs_are_valid(v in variable(), m in module_path(), s in unicode_string()) {
        assert!(v.starts_with('$') || v.starts_with('@') || v.starts_with('%'));
        assert!(!m.is_empty());
        assert!(s.is_char_boundary(s.len()));
    }
}
```

## Conventions

- Put reusable generators in `crates/perl-test-generators`
- Keep property tests in `tests/prop_*.rs`
- Save regression artifacts under `tests/_proptest-regressions/`
- Prefer `ProptestConfig` with file persistence for tests that can shrink

## Current Usage

The main current consumer is the UTF-16 roundtrip coverage in
[`crates/perl-position-tracking/tests/prop_utf16_roundtrip.rs`](../../crates/perl-position-tracking/tests/prop_utf16_roundtrip.rs).

The older corpus and parser generators in `perl-corpus` remain the right place
for parser-specific fixtures.
