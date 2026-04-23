# ADR-0012: Error Handling Strategy (No Panics Policy)

**Status**: Accepted
**Date**: 2025-01-10
**Decision Makers**: Perl LSP Architecture Team
**Related**: [AGENTS.md](../../AGENTS.md) - Coding Standards

## Context

The Perl LSP server is a long-running process that editors depend on for code intelligence. Unlike command-line tools that can exit on error, an LSP server must remain operational even when encountering unexpected conditions.

### Problem Statement

1. **Server Reliability**: LSP server crashes disrupt developer workflow
2. **Editor Integration**: Crashes may leave editors in inconsistent state
3. **Error Recovery**: Need graceful degradation, not abrupt termination
4. **Debugging**: Panics lose context that could aid debugging
5. **User Trust**: Frequent crashes erode confidence in the tool

### Failure Modes in LSP Context

| Failure Type | Example | Desired Behavior |
|--------------|---------|------------------|
| Parse Error | Malformed Perl syntax | Log error, return partial result |
| I/O Error | File deleted mid-parse | Log error, skip file |
| Protocol Error | Malformed LSP message | Log error, send error response |
| Logic Error | Unexpected state | Log error, continue with fallback |
| Resource Error | Out of memory | Log error, release resources |

## Decision

**We ban fatal constructs in production code: `unwrap()`, `expect()`, `panic!()`, `todo!()`, and `unimplemented()` are prohibited.**

### Banned Constructs

```rust
// ❌ BANNED in production code
let value = option.unwrap();
let value = result.expect("message");
panic!("unexpected state");
todo!("implement later");
unimplemented!();
```

### Required Alternatives

```rust
// ✅ Use ? operator for error propagation
let value = option.ok_or_else(|| Error::NotFound)?;

// ✅ Use pattern matching for explicit handling
match option {
    Some(value) => process(value),
    None => handle_missing(),
}

// ✅ Use ok_or for simple error conversion
let value = result.map_err(|e| Error::from(e))?;

// ✅ Use if let for optional handling
if let Some(value) = option {
    process(value);
}

// ✅ Use unwrap_or/unwrap_or_else for defaults
let value = option.unwrap_or(default_value);
let value = option.unwrap_or_else(|| compute_default());
```

### Test Code Exceptions

In test code, use `Result<()>` return types or the `perl_tdd_support` helpers:

```rust
// ✅ Tests can use Result return type
#[test]
fn test_something() -> Result<()> {
    let value = parse(input)?;
    assert_eq!(value, expected);
    Ok(())
}

// ✅ Tests can use must/must_some helpers
use perl_tdd_support::{must, must_some};

#[test]
fn test_with_helpers() {
    let value = must_some(parse(input)); // Panics on None with context
    let result = must(operation);        // Panics on Err with context
}
```

### Error Type Design

```rust
/// Unified error type for the LSP server
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("Parse error at {position}: {message}")]
    ParseError { position: usize, message: String },
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result alias for LSP operations
pub type Result<T> = std::result::Result<T, LspError>;
```

### Graceful Degradation Pattern

```rust
fn handle_request(&mut self, request: Request) -> Response {
    match self.process_request(request) {
        Ok(response) => response,
        Err(e) => {
            // Log the error with full context
            tracing::error!(
                error = %e,
                request_id = %request.id,
                "Request processing failed"
            );
            
            // Return error response, don't crash
            Response::error(request.id, e.to_string())
        }
    }
}
```

## Alternatives Considered

### Option 1: Allow Panics with Catch
**Description**: Use `std::panic::catch_unwind` to catch panics

**Pros**:
- Less code change required
- Can continue using unwrap/expect

**Cons**:
- Not guaranteed to catch all panics
- Loses panic context and stack trace
- Creates false sense of security
- Doesn't work with FFI boundaries

**Decision**: Rejected - unreliable and masks root causes

### Option 2: Result Types with Early Return
**Description**: Use Result everywhere with early `?` propagation

**Pros**:
- Idiomatic Rust
- Clear error flow
- Good error context

**Cons**:
- More verbose than unwrap
- Requires error type design
- Can lead to error type proliferation

**Decision**: Accepted - this is the chosen approach

### Option 3: Custom Panic Handler
**Description**: Install custom panic handler that logs and continues

**Pros**:
- Single point of handling
- Can add custom logging

**Cons**:
- Cannot actually continue after panic
- Stack unwinding may be incomplete
- Still results in server termination

**Decision**: Rejected - doesn't solve the core problem

## Consequences

### Positive

1. **Server Reliability**:
   - Server remains operational despite errors
   - Editors maintain connection
   - Developer workflow uninterrupted

2. **Better Error Context**:
   - Errors include contextual information
   - Stack traces preserved through logging
   - Easier debugging of production issues

3. **Explicit Error Handling**:
   - All error paths are visible in code
   - Code reviewers see error handling
   - No hidden failure modes

4. **Graceful Degradation**:
   - Partial results when possible
   - Clear error messages to users
   - Fallback behaviors documented

5. **Predictable Behavior**:
   - No surprise terminations
   - Consistent error responses
   - Testable error conditions

### Negative

1. **More Verbose Code**:
   - Error handling adds code volume
   - More boilerplate for simple operations
   - Can obscure main logic flow

2. **Error Type Complexity**:
   - Need to define error types
   - Error conversion implementations
   - Potential for error type explosion

3. **Learning Curve**:
   - New contributors must learn patterns
   - Different from simple Rust examples
   - Requires understanding of error design

4. **Test Helper Dependency**:
   - Tests need special helpers
   - Additional crate dependency
   - Must maintain test utilities

### Mitigations

1. **Error Type Macros**:
   ```rust
   // Use thiserror to reduce boilerplate
   #[derive(Debug, thiserror::Error)]
   pub enum Error {
       #[error("...")]
       Variant(...),
   }
   ```

2. **Result Aliases**:
   ```rust
   // Standardize result types
   pub type Result<T> = std::result::Result<T, Error>;
   ```

3. **Helper Functions**:
   ```rust
   // Common patterns as utilities
   fn required<T>(option: Option<T>) -> Result<T> {
       option.ok_or_else(|| Error::MissingRequired)
   }
   ```

4. **Documentation**:
   - Clear examples in coding standards
   - Code review checklist includes error handling
   - Onboarding materials cover patterns

## Enforcement

### CI Checks

```yaml
# .github/workflows/lint.yml
- name: Check for banned constructs
  run: |
    # Check for unwrap in production code
    ! grep -rn "\.unwrap()" --include="*.rs" crates/ | grep -v test
    # Check for expect in production code
    ! grep -rn "\.expect(" --include="*.rs" crates/ | grep -v test
    # Check for panic!
    ! grep -rn "panic!" --include="*.rs" crates/ | grep -v test
```

### Clippy Lints

```toml
# .clippy.toml
disallowed-methods = [
    { path = "std::option::Option::unwrap", reason = "Use ok_or_else instead" },
    { path = "std::result::Result::unwrap", reason = "Use ? operator instead" },
    { path = "std::option::Option::expect", reason = "Use ok_or_else with context" },
    { path = "std::result::Result::expect", reason = "Use map_err with context" },
]
```

### Code Review Checklist

- [ ] No `unwrap()` in production code
- [ ] No `expect()` in production code
- [ ] No `panic!()` in production code
- [ ] No `todo!()` in production code
- [ ] No `unimplemented!()` in production code
- [ ] All errors properly propagated or handled
- [ ] Error messages include useful context
- [ ] Tests use `Result<()>` or test helpers

## Additional Guidelines

### Prefer .first() over .get(0)

```rust
// ❌ Less clear
let first = slice.get(0);

// ✅ More idiomatic
let first = slice.first();
```

### Use .push(char) for Single Characters

```rust
// ❌ Less efficient
buffer.push_str("x");

// ✅ More efficient
buffer.push('x');
```

### Use or_default() for Vec Initialization

```rust
// ❌ More verbose
map.entry(key).or_insert_with(Vec::new)

// ✅ More concise
map.entry(key).or_default()
```

### Avoid Unnecessary Clone on Copy Types

```rust
// ❌ Unnecessary
let x: u32 = y.clone();

// ✅ Copy is implicit
let x: u32 = y;
```

## References

- [Rust Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [thiserror crate](https://docs.rs/thiserror)
- [anyhow crate](https://docs.rs/anyhow)
- [AGENTS.md - Coding Standards](../../AGENTS.md)
