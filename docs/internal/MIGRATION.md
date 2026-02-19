# Migration Guide

## v0.8.10+ Documentation Infrastructure Changes

### API Documentation Requirements (SPEC-149)

Starting with v0.8.10+, the perl-parser crate enforces `#![warn(missing_docs)]` for all public APIs. This is **non-breaking** for consumers but introduces new development requirements.

#### For Downstream Consumers

**No immediate action required** - this change only affects development builds when extending perl-parser APIs.

```rust
// This will now produce compilation warnings in development:
pub fn my_parser_extension() -> Result<Ast, ParseError> {
    // Missing documentation comment above
}

// Fixed version with proper documentation:
/// Extends the parser with custom functionality for specialized Perl parsing.
///
/// # Arguments
/// * `input` - The Perl source code to parse
///
/// # Returns
/// * `Ok(Ast)` - Successfully parsed AST
/// * `Err(ParseError)` - Parsing failed due to syntax errors
///
/// # Examples
/// ```rust
/// use perl_parser::Parser;
/// let result = my_parser_extension("my $var = 42;")?;
/// assert!(result.is_valid());
/// ```
pub fn my_parser_extension() -> Result<Ast, ParseError> {
    // Implementation
}
```

#### For Library Contributors

**Action required** if contributing to perl-parser crate:

1. **Follow API Documentation Standards**: All public APIs must include comprehensive documentation
2. **Include LSP Workflow Context**: Document how your changes fit into Parse → Index → Navigate → Complete → Analyze pipeline
3. **Add Performance Documentation**: Critical modules must document memory usage and large Perl file processing characteristics

#### Timeline and Scope

- **Current State**: 603 missing documentation warnings identified
- **Implementation Strategy**: Phased approach over multiple releases
- **Phase 1**: Core parser APIs (parser.rs, ast.rs, error.rs)
- **Phase 2**: LSP provider interfaces (completion.rs, diagnostics.rs, workspace_index.rs)
- **Phase 3**: Specialized features (import_optimizer.rs, test_generator.rs, scope_analyzer.rs)

#### Validation and Testing

The documentation infrastructure includes comprehensive testing:

```bash
# Validate documentation standards compliance
cargo test -p perl-parser --test missing_docs_ac_tests

# Check for specific acceptance criteria
cargo test -p perl-parser --test missing_docs_ac_tests -- --nocapture test_missing_docs_warning_compilation

# Generate documentation without warnings (requires complete docs)
cargo doc --no-deps --package perl-parser
```

#### Integration with Existing Standards

This change integrates with the existing [API Documentation Standards](docs/API_DOCUMENTATION_STANDARDS.md):

- **Enterprise-grade documentation requirements** maintained
- **TDD validation methodology** for documentation completeness
- **LSP workflow integration** documentation patterns
- **Performance-critical module** documentation requirements

## v0.8.0 Breaking Changes

### DeclarationProvider API Change

The `find_declaration()` method now requires version tracking to prevent stale provider reuse after AST refresh.

#### Before (v0.7.x)
```rust
let provider = DeclarationProvider::new(&tree);
let location = provider.find_declaration(offset, col);
```

#### After (v0.8.0)
```rust
let provider = DeclarationProvider::new(&tree);
let current_version = provider.version();
let location = provider.find_declaration(offset, col, current_version);
```

Or use the convenience method:
```rust
let provider = DeclarationProvider::new(&tree);
let location = provider.with_doc_version(doc_version)
    .find_declaration(offset, col, doc_version);
```

### Why This Change?

The version parameter ensures the DeclarationProvider's cached AST matches the current document state. This prevents:
- Incorrect jump targets after edits
- Stale cache usage across document versions  
- Race conditions in concurrent LSP requests

### LineStartsCache Performance

The new `LineStartsCache` provides 40-100x faster position conversions:
- 1M lines: 7-20µs (was ~1ms)
- Zero overhead in release builds
- Thread-safe with Arc

```rust
use perl_parser::positions::LineStartsCache;

let cache = LineStartsCache::new(content);
let (line, col) = cache.offset_to_position(content, offset);
let offset = cache.position_to_offset(content, line, col);
```

Note: The `positions` module is marked `#[doc(hidden)]` as it's considered semi-internal.

## v0.7.x to v0.8.0

### LSP Server

No changes needed for LSP server users. The server internally handles the API change.

### Library Users

If you're using perl-parser as a library:

1. **Update DeclarationProvider calls** - Add version parameter
2. **Consider LineStartsCache** - For faster position conversions
3. **Review semi-internal APIs** - `positions` module is now `#[doc(hidden)]`

### Extension Users

The VSCode extension now includes:
- Auto-download of LSP binary
- SHA256 verification
- Platform detection
- Progress notifications

## v0.6.x to v0.7.x

### Type System Enhancements

Hash literal type inference now uses smart unification:
```perl
my $config = { 
    port => 8080,      # Inferred as Numeric
    host => 'localhost' # Inferred as String
};
```

### Workspace File Operations

New support for file watching and multi-file refactoring:
- `didChangeWatchedFiles` notifications
- `willRenameFiles` requests
- Cross-file symbol updates

## Common Issues

### Compilation Errors

If you see "missing argument" errors after upgrading:
```
error[E0061]: this method takes 3 arguments but 2 arguments were supplied
```

Add the version parameter to `find_declaration()` calls.

### Performance Considerations

The new position cache allocates ~8 bytes per line. For massive files (>10M lines), consider:
- Streaming processing
- Line-by-line analysis
- Document splitting

### CRLF Handling

Known limitation: `\r` in CRLF sequences may not round-trip perfectly through position conversions. This is documented and tested but not fully resolved.

## Getting Help

- **Upgrade issues**: [GitHub Issues](https://github.com/EffortlessMetrics/tree-sitter-perl/issues)
- **API questions**: See rustdoc: `cargo doc --open`
- **Performance**: Run benchmarks: `cargo bench`