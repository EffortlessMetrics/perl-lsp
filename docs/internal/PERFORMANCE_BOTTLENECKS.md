# Performance Bottlenecks Analysis - Lexer+Pest Parser

## Current Architecture Overhead

The multi-phase parser has several performance bottlenecks:

### 1. **String Allocations & Copying**

#### Problem Areas:
```rust
// lexer_adapter.rs - Multiple string allocations
let mut result = String::with_capacity(input.len() * 2);  // Over-allocating
result.push_str(&input[last_end..token.start]);           // Copying unchanged text
result.push_str("_DIV_");                                  // Multiple small allocations
```

#### Optimization:
- Use a single-pass approach with in-place modifications
- Pre-calculate exact capacity needed
- Use `Cow<str>` to avoid copying when no changes needed

### 2. **Multiple Parsing Passes**

Current flow:
1. Heredoc parsing (full scan + string copy)
2. Lexer preprocessing (full scan + string copy)
3. Pest parsing (full scan)
4. AST building (tree traversal)
5. Postprocessing (tree traversal)

**Total: 3 full scans + 2 tree traversals**

### 3. **Inefficient Token Replacement**

```rust
// Current approach
"10 / 2" → "10 _DIV_ 2"  // 7 chars → 11 chars
"s/a/b/" → "_SUB_/a/b/"   // 7 chars → 11 chars
```

This causes:
- String reallocations
- Offset tracking complexity
- Grammar complexity for `_DIV_` tokens

### 4. **Pest Parser Overhead**

- PEG parsing has inherent overhead
- Backtracking on failed alternatives
- Creating intermediate parse tree before AST

### 5. **Debug Output in Release Builds**

```rust
eprintln!("Pest parse error: {:?}", e);
eprintln!("Input after preprocessing: {:?}", fully_processed);
```

These should be behind `#[cfg(debug_assertions)]`

## Optimization Strategies

### 1. **Single-Pass Lexer+Parser**

Instead of preprocessing, make the Pest grammar context-aware:
```pest
// Add lexer state to grammar
slash = { 
    division_context ~ "/" |
    regex_context ~ "/" ~ regex_body ~ "/" ~ regex_flags?
}
```

### 2. **Zero-Copy Parsing**

Use slices and indices instead of copying:
```rust
enum Token<'a> {
    Division { slice: &'a str },
    Regex { pattern: &'a str, flags: &'a str },
}
```

### 3. **Streaming Parser**

Parse as we lex, don't build intermediate strings:
```rust
struct StreamingParser<'a> {
    input: &'a str,
    lexer: PerlLexer<'a>,
    ast_builder: AstBuilder,
}
```

### 4. **Optimize Common Cases**

Fast path for files without:
- Heredocs (90% of files)
- Complex slash ambiguity (80% of files)
- Unicode (95% of files)

### 5. **Better Memory Layout**

```rust
// Current: Arc<str> everywhere
pub enum AstNode {
    String(Arc<str>),
    // ...
}

// Optimized: Use &'a str where possible
pub enum AstNode<'a> {
    String(&'a str),  // No allocation for literals
    // ...
}
```

### 6. **Lazy Evaluation**

Don't process parts we don't need:
```rust
// Only parse function bodies when needed
SubDeclaration {
    name: &'a str,
    body: LazyParse<'a>,  // Parse on demand
}
```

## Estimated Performance Gains

1. **Remove string copies**: 30-40% faster
2. **Single-pass parsing**: 20-30% faster
3. **Zero-copy AST**: 15-20% faster
4. **Fast paths**: 10-15% faster for common cases
5. **Remove debug output**: 5-10% faster

**Total potential speedup: 2-3x**

This would bring the Rust parser to:
- ~60-100 µs/KB (from 180-200 µs/KB)
- ~0.4-0.6ms total time (from 1.3ms)

## Implementation Priority

1. **Quick wins** (1-2 hours):
   - Remove debug output in release
   - Pre-allocate exact string capacity
   - Add fast path for simple files

2. **Medium effort** (1-2 days):
   - Implement zero-copy token representation
   - Optimize AST memory layout
   - Combine heredoc + lexer passes

3. **Major refactor** (1 week):
   - Single-pass streaming parser
   - Context-aware Pest grammar
   - Lazy parsing for large constructs

With these optimizations, the Rust parser could potentially **match or exceed** C performance while maintaining safety and correctness.