# ADR-94d78475: Add utf8::encode/utf8::decode Builtin Documentation

## Status
Proposed

## Context

GitHub issue #3371 reports that perl-lsp does not provide hover documentation for `utf8::encode()` and `utf8::decode()` functions, which are core Perl builtins for explicit UTF-8 encoding/decoding. The LSP should recognize these functions and display appropriate hover documentation.

Investigation revealed:
- The lexer's PHF tables already recognize all 8 utf8 functions (encode, decode, is_utf8, valid, upgrade, downgrade, native_to_unicode, unicode_to_native)
- The semantic analyzer's `get_builtin_documentation()` in `builtins.rs` has no entries for any utf8 functions
- The hover flow correctly calls `get_builtin_documentation()` for `FunctionCall` nodes with qualified names like `"utf8::encode"`
- This is purely a documentation gap — the infrastructure already exists

## Decision

Add hover documentation entries for three utf8 functions to `get_builtin_documentation()` in `builtins.rs`:
- `utf8::encode` — converts string from Unicode to UTF-8 encoded bytes
- `utf8::decode` — converts string from UTF-8 encoded bytes to Unicode
- `utf8::downgrade` — attempts to convert Unicode string to bytes (fails if characters beyond U+00FF)

Each entry uses the existing `BuiltinDoc` struct with only `signature` and `description` fields.

## Alternatives Considered

### 1. Add all 8 utf8 functions
The lexer has all 8 utf8 functions registered. Adding all of them was considered but rejected because:
- Issue #3371 specifically requests only encode/decode/downgrade
- Other functions (upgrade, valid, is_utf8) are less commonly used in isolation
- Scope should be limited to match the issue request

### 2. Add version_required metadata
The plan proposed using `version_required: Some("v5.6")` to indicate Perl version requirements. This was rejected because:
- `BuiltinDoc` only has `signature` and `description` fields — no `version_required`
- That field only exists on `PragmaDoc`
- Including it would be a compilation error

### 3. Implement encoding state tracking
The issue mentions tracking variable encoding state after calls. This was explicitly excluded because:
- Requires complex type inference
- Beyond the scope of a documentation-only fix
- Can be addressed in a future enhancement

## Consequences

### Benefits
- Users get hover documentation for commonly-used utf8 functions
- Fills gap between lexer's builtin recognition and semantic analyzer's hover
- Low risk — documentation-only change to a match statement
- Follows existing patterns used by 80+ other builtin entries

### Tradeoffs
- Only covers three of the eight utf8 functions (lexer's full set not included)
- No encoding state tracking (scope limitation per issue)

### Risks
- Low: Simple addition to existing match statement
- Name collision: Another module could define `utf8::encode`, but this is a core Perl function so documentation is correct