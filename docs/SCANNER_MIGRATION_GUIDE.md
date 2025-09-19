# Scanner Migration Guide

This guide documents the unified scanner architecture and migration from C-based to Rust-based scanning in the Perl parsing ecosystem.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Unified Scanner Design](#unified-scanner-design)
- [Migration Strategy](#migration-strategy)
- [Feature Gates](#feature-gates)
- [Backward Compatibility](#backward-compatibility)
- [Performance Considerations](#performance-considerations)
- [Testing and Validation](#testing-and-validation)
- [Troubleshooting](#troubleshooting)

## Architecture Overview

The scanner implementation employs a unified Rust-based architecture with C compatibility wrapper, providing seamless migration from legacy C implementations while maintaining performance and compatibility.

### Key Components

#### Rust Scanner (`RustScanner`)
- **Core Implementation**: Native Rust scanner with full Perl lexical analysis
- **Performance**: Optimized for speed with keyword caching and efficient tokenization
- **Unicode Support**: Full Unicode identifier and emoji support with UTF-8/UTF-16 handling
- **Memory Safety**: Rust's ownership model eliminates common scanner vulnerabilities

#### C Scanner Wrapper (`CScanner`)
- **Compatibility Layer**: Delegates to `RustScanner` for legacy API support
- **API Preservation**: Maintains existing C scanner interface contracts
- **Resource Management**: Safe cleanup of C scanner resources through Drop trait
- **Transition Bridge**: Allows gradual migration from C-based implementations

#### Unified Implementation
- **Single Source**: Both scanner features (`c-scanner` and `rust-scanner`) use the same Rust code
- **Reduced Maintenance**: Single implementation reduces overhead while preserving API contracts
- **Consistent Behavior**: Ensures identical tokenization results across scanner implementations

## Unified Scanner Design

### Scanner Trait Interface

```rust
pub trait PerlScanner {
    /// Scan the next token from the input
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>>;

    /// Serialize the scanner state
    fn serialize(&self, buffer: &mut Vec<u8>) -> ParseResult<()>;

    /// Deserialize the scanner state
    fn deserialize(&mut self, buffer: &[u8]) -> ParseResult<()>;

    /// Check if the scanner is at the end of input
    fn is_eof(&self) -> bool;

    /// Get the current position in the input
    fn position(&self) -> (usize, usize);

    /// Check if we're in a regex context
    fn is_regex_context(&self) -> bool;
}
```

### Delegation Pattern

The C scanner wrapper delegates all operations to the underlying Rust implementation:

```rust
impl PerlScanner for CScanner {
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>> {
        // Delegates to RustScanner internally
        self.rust_scanner.scan(input)
    }

    // Additional methods follow the same delegation pattern
}
```

## Migration Strategy

### Phase 1: C Scanner Compatibility
- Maintain existing C scanner interface
- Implement Rust scanner as alternative option
- Provide feature flags for selecting implementation
- Ensure test compatibility across both implementations

### Phase 2: Unified Implementation
- Implement delegation pattern in C scanner wrapper
- Route all scanner operations through Rust implementation
- Maintain API compatibility for existing code
- Validate performance parity or improvement

### Phase 3: Legacy Deprecation
- Mark C-specific implementations as legacy
- Encourage migration to Rust scanner
- Provide migration tools and documentation
- Maintain backward compatibility for one major version

## Feature Gates

### Scanner Feature Configuration

```toml
[features]
default = ["pure-rust"]
pure-rust = []
pure-rust-standalone = ["pure-rust"]  # For benchmarking without perl-lexer

# Legacy features (for benchmarking only)
c-scanner = []
rust-scanner = []
```

### Usage Examples

#### Rust Scanner (Recommended)
```rust
#[cfg(feature = "rust-scanner")]
use tree_sitter_perl::scanner::RustScanner;

let mut scanner = RustScanner::new();
let result = scanner.scan(input)?;
```

#### C Scanner (Legacy Compatibility)
```rust
#[cfg(feature = "c-scanner")]
use tree_sitter_perl::scanner::CScanner;

let mut scanner = CScanner::new();
let result = scanner.scan(input)?; // Delegates to Rust internally
```

#### Feature-Agnostic Code
```rust
use tree_sitter_perl::scanner::PerlScanner;

fn create_scanner() -> Box<dyn PerlScanner> {
    #[cfg(feature = "rust-scanner")]
    return Box::new(RustScanner::new());

    #[cfg(feature = "c-scanner")]
    return Box::new(CScanner::new());
}
```

## Backward Compatibility

### API Preservation
- All existing C scanner APIs remain functional
- Function signatures and return types unchanged
- Error handling behavior consistent
- State serialization/deserialization compatible

### Test Suite Compatibility
```rust
// Existing benchmark code continues to work
let mut scanner = create_scanner();
let input = black_box(test_case.as_bytes());
scanner.scan(input).unwrap();
```

### Build System Integration
- Cargo features preserve existing build configurations
- No changes required to existing build scripts
- Conditional compilation maintains compatibility

## Performance Considerations

### Optimization Benefits
- **Keyword Caching**: Pre-populated HashMap for O(1) keyword lookup
- **Memory Efficiency**: Rust's zero-cost abstractions and ownership model
- **Unicode Performance**: Optimized UTF-8/UTF-16 handling
- **Branch Prediction**: Structured token type matching

### Benchmarking Support
```bash
# Compare scanner implementations
cargo bench --bench scanner_benchmarks --features rust-scanner
cargo bench --bench scanner_benchmarks --features c-scanner

# Feature-specific benchmarking
cargo run --features parser-tasks -- highlight  # Tree-sitter integration
```

### Performance Metrics
- **Token Generation**: Consistent performance across implementations
- **Memory Usage**: Reduced allocation overhead with Rust scanner
- **Throughput**: Maintains or improves parsing speed
- **Latency**: Predictable tokenization times

## Testing and Validation

### Comprehensive Test Coverage
```bash
# Test both scanner implementations
cargo test --features rust-scanner
cargo test --features c-scanner

# Cross-implementation validation
cargo test --test scanner_compatibility
```

### Integration Tests
- Tree-sitter parser integration
- Perl corpus validation
- Edge case handling
- Error recovery scenarios

### Regression Testing
- Existing test suites continue to pass
- Performance benchmarks maintain baselines
- API compatibility verification
- State serialization round-trip tests

## Troubleshooting

### Common Migration Issues

#### Feature Flag Conflicts
```toml
# Problem: Both features enabled
[features]
default = ["rust-scanner", "c-scanner"]  # Conflict

# Solution: Use one feature at a time
[features]
default = ["rust-scanner"]
```

#### Compilation Errors
```rust
// Problem: Missing feature conditional
let scanner = RustScanner::new();  // Error without rust-scanner feature

// Solution: Use feature gates
#[cfg(feature = "rust-scanner")]
let scanner = RustScanner::new();
```

#### Performance Regression
```rust
// Problem: Using boxed trait objects in hot paths
let scanner: Box<dyn PerlScanner> = create_scanner();

// Solution: Use concrete types when possible
let scanner = RustScanner::new();
```

### Diagnostic Tools

#### Scanner State Inspection
```rust
let scanner = RustScanner::new();
println!("Position: {:?}", scanner.position());
println!("EOF: {}", scanner.is_eof());
println!("Regex context: {}", scanner.is_regex_context());
```

#### Debug Logging
```rust
let config = ScannerConfig {
    debug: true,
    ..Default::default()
};
let scanner = RustScanner::with_config(config);
```

### Migration Checklist

- [ ] Review current scanner usage patterns
- [ ] Enable `rust-scanner` feature flag
- [ ] Run comprehensive test suite
- [ ] Validate performance benchmarks
- [ ] Update documentation and examples
- [ ] Plan deprecation timeline for C scanner
- [ ] Monitor production metrics post-migration

## Advanced Configuration

### Scanner Configuration Options
```rust
pub struct ScannerConfig {
    /// Enable strict mode for better error reporting
    pub strict_mode: bool,
    /// Enable Unicode normalization
    pub unicode_normalization: bool,
    /// Maximum token length to prevent DoS
    pub max_token_length: usize,
    /// Enable debug logging
    pub debug: bool,
}
```

### Custom Scanner Implementation
```rust
impl PerlScanner for CustomScanner {
    fn scan(&mut self, input: &[u8]) -> ParseResult<Option<u16>> {
        // Custom scanning logic
        // Can delegate to RustScanner for standard cases
        self.rust_scanner.scan(input)
    }
}
```

## Conclusion

The unified scanner architecture provides a seamless migration path from C-based to Rust-based scanning while maintaining full backward compatibility. The delegation pattern ensures that existing code continues to work without modification, while new development can take advantage of Rust's performance and safety benefits.

This migration strategy supports gradual adoption, comprehensive testing, and performance validation, making it suitable for production environments with strict compatibility requirements.