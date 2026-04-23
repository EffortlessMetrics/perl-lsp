# perl-test-must

Panic-on-failure helpers for Rust tests.

## Problem it solves

This workspace bans `unwrap()` and `expect()` in production code, but tests
still need concise ways to express "this must succeed". This crate provides
small helpers that intentionally panic on failure so test code stays readable
without violating the workspace lint policy.

## Public API

- `must` extracts the value from a `Result`
- `must_some` extracts the value from an `Option`
- `must_err` extracts the error from a `Result`

## Example

```rust
use perl_test_must::{must, must_err, must_some};

let value: Result<i32, &str> = Ok(42);
assert_eq!(must(value), 42);

let item = Some("ok");
assert_eq!(must_some(item), "ok");

let err: Result<i32, &str> = Err("boom");
assert_eq!(must_err(err), "boom");
```

## Workspace role

Used in tests across the workspace as the policy-compliant replacement for
`unwrap`, `unwrap_err`, and `expect`.

## License

MIT OR Apache-2.0
