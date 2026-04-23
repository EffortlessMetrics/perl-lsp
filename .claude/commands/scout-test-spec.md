---
description: Scout step 6 — write the exact test code that proves the fix works
user-invocable: false
---

# Scout Test Spec

Write the test that a builder will add. This is actual code, not a description.

## For parser fixes

```rust
#[test]
fn test_<descriptive_name>() {
    // The minimal Perl code from step 3
    let perl = r#"<your reproduction snippet>"#;
    let result = parse_perl_document(perl);
    assert!(result.errors.is_empty(), "Expected clean parse, got: {:?}", result.errors);
}
```

## For LSP features

```rust
#[test]
fn test_<feature_name>() {
    // Setup: create a document with specific content
    // Action: send the LSP request
    // Assert: response matches expected
}
```

## Verification command

```bash
cargo test -p <crate> -- <test_name> --exact
cargo xtask fmt && cargo clippy -p <crate> --tests
```

## Output

Record in your task:
```
Test file: crates/<crate>/tests/<name>.rs
Test function: test_<name>
Test code: <the actual Rust code>
Verify: cargo test -p <crate> -- test_<name> --exact
```

## Rule

If you can't write the test code, you don't understand the problem well enough.
Go back to step 3 (reproduce).
