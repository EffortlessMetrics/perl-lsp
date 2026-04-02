# perl-dap-value

Shared Perl value model for debugger rendering.

This crate models the values that show up in the variables pane: scalars,
arrays, hashes, refs, objects, tied values, truncation, and inspection errors.
It is the value layer under the DAP transport types.

## Boundaries

- Use `perl-dap-types` for stack frames, sources, and variables.
- Use `perl-dap-value` when you need to describe what a variable actually
  contains.
- Use `perl-dap` when you need to serialize that model into DAP responses.

## Key type

- `PerlValue`

## Example

```rust
use perl_dap_value::PerlValue;

let value = PerlValue::object(
    "My::Class",
    PerlValue::array(vec![PerlValue::scalar("alpha"), PerlValue::Integer(42)]),
);

assert!(value.is_expandable());
assert_eq!(value.type_name(), "OBJECT");
```
