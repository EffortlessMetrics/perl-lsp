# Debug Build Stack Overflow Fix

## Problem
The Pure Rust parser experiences stack overflow in debug builds when parsing deeply nested Perl structures (depth >500-1000). This doesn't occur in release builds due to optimizations.

## Root Cause
The `build_node` function in `pure_rust_parser.rs` uses recursion to build the AST. Each nested structure adds a stack frame, eventually exhausting the default stack size in debug builds.

## Implemented Solution: Stacker Integration

### 1. Added `stacker` crate dependency
```toml
# Cargo.toml
stacker = "0.1"
```

### 2. Wrapped recursive function with stack growth
```rust
pub(crate) fn build_node(&mut self, pair: Pair<Rule>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
    const STACK_RED_ZONE: usize = 256 * 1024; // 256KB
    const STACK_SIZE: usize = 2 * 1024 * 1024; // 2MB growth
    stacker::maybe_grow(STACK_RED_ZONE, STACK_SIZE, || {
        self.build_node_impl(pair)
    })
}
```

The original `build_node` is renamed to `build_node_impl` and all recursive calls go through the wrapper.

## Testing
Created test cases in:
- `tests/debug_stack_overflow_test.rs` - Comprehensive tests for different nesting patterns
- `tests/test_stacker_fix.rs` - Simple verification tests

## Status
- ✅ Stacker integration complete
- ✅ Test cases created
- ⚠️ Build/test verification pending (compilation taking long time)

## Alternative Solutions (Future)

### 1. Iterative AST Building
Convert the recursive `build_node` to use an explicit stack:
```rust
fn build_node_iterative(&mut self, initial_pair: Pair<Rule>) -> Result<Option<AstNode>, Box<dyn std::error::Error>> {
    let mut stack: Vec<BuildState> = vec![BuildState::new(initial_pair)];
    let mut results: Vec<AstNode> = Vec::new();
    
    while let Some(state) = stack.pop() {
        // Process node without recursion
    }
}
```

### 2. Tail Call Optimization
Restructure recursive calls to be tail-recursive where possible.

### 3. Streaming Parser
Process AST nodes as they're built rather than building the entire tree in memory.

## Next Steps
1. Verify the stacker fix works in practice
2. Consider implementing iterative AST building for better performance
3. Add CI tests that run with limited stack size to catch regressions