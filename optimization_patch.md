# Performance Optimization Patch for Pure Rust Parser

## Summary of Optimizations

Based on the benchmark results showing a 14% performance gap, here are the key optimizations to implement:

### 1. Grammar Optimizations (High Impact)

**Use Atomic Rules (`@`) to Prevent Backtracking:**

```pest
// Before
identifier = { (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }

// After  
identifier = @{ (ASCII_ALPHA | "_") ~ (ASCII_ALPHANUMERIC | "_")* }
```

Apply atomic rules to:
- All variable patterns (`scalar_variable`, `array_variable`, etc.)
- Operators (they have fixed format)
- Numbers and basic literals
- Comments

### 2. Reduce String Allocations (High Impact)

**Current code has 50+ `.to_string()` calls. Replace with:**

```rust
// Before
let scope = inner.next().unwrap().as_str().to_string();

// After (use &str where possible)
let scope = inner.next().unwrap().as_str();

// Or use Arc<str> for shared ownership
let scope: Arc<str> = Arc::from(inner.next().unwrap().as_str());
```

### 3. Add Inline Hints (Medium Impact)

```rust
#[inline(always)]
pub fn parse(&mut self, input: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
    // ...
}

#[inline]
pub(crate) fn build_node(&mut self, pair: Pair<Rule>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
    // ...
}

#[inline]
fn build_binary_chain(&mut self, pairs: Vec<Pair<Rule>>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
    // ...
}
```

### 4. Fast Paths for Common Constructs (Medium Impact)

Add dedicated rules for the most common Perl patterns:

```pest
// Fast path for simple assignments
simple_assignment = @{ variable ~ "=" ~ (literal | variable) ~ semicolon }

// Fast path for method calls  
simple_method_call = @{ variable ~ "->" ~ identifier ~ "()" ~ semicolon }

// Fast path for array/hash access
simple_array_access = @{ variable ~ "[" ~ (number | variable) ~ "]" }
simple_hash_access = @{ variable ~ "{" ~ (string | variable) ~ "}" }
```

### 5. Pre-allocate Vectors (Low Impact)

```rust
// Before
let mut statements = Vec::new();

// After (for typical files)
let mut statements = Vec::with_capacity(32);
```

### 6. Reduce Clone Operations

```rust
// Before (in build_ternary)
let condition = Box::new(self.build_node(inner[0].clone())?.unwrap());
let then_expr = Box::new(self.build_node(inner[1].clone())?.unwrap());

// After (avoid cloning Pairs)
let mut inner_iter = inner.into_iter();
let condition = Box::new(self.build_node(inner_iter.next().unwrap())?.unwrap());
let then_expr = Box::new(self.build_node(inner_iter.next().unwrap())?.unwrap());
```

## Implementation Priority

1. **String allocation reduction** - Biggest win, easiest to implement
2. **Atomic grammar rules** - Prevents backtracking, moderate effort  
3. **Inline hints** - Easy to add, measurable impact
4. **Fast paths** - More complex but helps common cases
5. **Vector pre-allocation** - Minor improvement

## Expected Performance Gains

- String optimizations: 5-8% improvement
- Atomic rules: 3-5% improvement  
- Inline hints: 2-3% improvement
- Fast paths: 2-4% improvement on typical code
- Combined: Should close the gap to within 5% of C parser

## Quick Wins (Do These First)

1. Replace all `.to_string()` with `Arc<str>` or `&str` where possible
2. Add `#[inline]` to `build_node`, `parse`, and helper methods
3. Make all token rules atomic with `@` prefix
4. Pre-allocate vectors with typical sizes

These optimizations maintain code clarity while significantly improving performance.