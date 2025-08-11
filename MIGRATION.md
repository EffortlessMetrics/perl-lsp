# Migration Guide

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

- **Upgrade issues**: [GitHub Issues](https://github.com/EffortlessSteven/tree-sitter-perl/issues)
- **API questions**: See rustdoc: `cargo doc --open`
- **Performance**: Run benchmarks: `cargo bench`