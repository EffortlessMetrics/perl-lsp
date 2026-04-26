# Spec — work-94d78475: utf8::encode/utf8::decode Builtin Documentation

## Feature/Behavior Description

Add hover documentation support for `utf8::encode()`, `utf8::decode()`, and `utf8::downgrade()` Perl builtins in the semantic analyzer. When a user hovers over a call to any of these functions, the LSP should display the function signature and a description of what the function does.

## Acceptance Criteria

1. **Hover on utf8::encode shows documentation**
   - When the cursor is on a call to `utf8::encode()`, the hover response contains the function signature and description indicating it converts a string from Unicode to UTF-8 encoded bytes
   - `get_builtin_documentation("utf8::encode")` returns a `BuiltinDoc` with `signature` and `description` fields

2. **Hover on utf8::decode shows documentation**
   - When the cursor is on a call to `utf8::decode()`, the hover response contains the function signature and description indicating it converts a string from UTF-8 encoded bytes to Unicode
   - `get_builtin_documentation("utf8::decode")` returns a `BuiltinDoc` with `signature` and `description` fields

3. **Hover on utf8::downgrade shows documentation**
   - When the cursor is on a call to `utf8::downgrade()`, the hover response contains the function signature and description
   - `get_builtin_documentation("utf8::downgrade")` returns a `BuiltinDoc` with `signature` and `description` fields

4. **Tests pass**
   - `cargo test -p perl-semantic-analyzer` passes
   - `cargo clippy -p perl-semantic-analyzer` passes with no warnings

## Non-Goals

- Variable encoding state tracking after utf8 function calls
- Type inference for encoding-aware operations
- Adding other utf8 functions (valid_utf8, upgrade, is_utf8_string, etc.)
- Warning for double-encoding issues

## Dependencies

- The existing hover infrastructure at `node_analysis.rs:254` which calls `get_builtin_documentation(name)` for `FunctionCall` nodes
- The AST already parses qualified function names like `"utf8::encode"` as `FunctionCall { name: "utf8::encode", args: [...] }`