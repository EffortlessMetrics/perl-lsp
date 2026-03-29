# perl-dap-value

[![Crates.io](https://img.shields.io/crates/v/perl-dap-value.svg)](https://crates.io/crates/perl-dap-value)
[![Documentation](https://docs.rs/perl-dap-value/badge.svg)](https://docs.rs/perl-dap-value)

Shared Perl runtime value model for debugger and renderer crates.

## When to use this crate

Use `perl-dap-value` when you need to represent Perl runtime values in a form
that can be serialized across the debugger stack.

It is the right crate for:

- debugger variable inspection payloads
- renderer or formatter code that needs a stable value enum
- tests or adapters that need to compare Perl values structurally

## Quick example

```rust
use perl_dap_value::PerlValue;

let value = PerlValue::object("My::Class", PerlValue::scalar("hello"));
assert_eq!(value.type_name(), "OBJECT");
assert!(value.is_expandable());
```

## Public API

- `PerlValue`: shared enum for `undef`, scalar, array, hash, reference, object, and error shapes
- `PerlValue::type_name`: returns the debugger-facing type label
- `PerlValue::child_count`: reports child counts for expandable values

## Workspace role

This is a small shared model crate for the DAP stack. It is usable on its own,
but its main job is to keep debugger value shapes consistent across parser,
transport, and renderer code.
