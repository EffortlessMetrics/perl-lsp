# perl-position-tracking

Position tracking and conversion utilities for the Perl LSP ecosystem.

## Overview

This crate provides foundational types for source location tracking, enabling accurate position conversion between byte offsets (used by parsers) and line/column positions (used by LSP clients).

## Features

- **ByteSpan**: Byte-offset based spans for parser/AST use
- **LineStartsCache**: Efficient line index for offset-to-position conversion
- **UTF-16/UTF-8 Conversion**: Symmetric position conversion for LSP protocol compliance
- **WirePosition/WireRange**: LSP protocol-compatible position types (with `lsp-compat` feature)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
perl-position-tracking = "0.8.8"

# For LSP protocol types
perl-position-tracking = { version = "0.8.8", features = ["lsp-compat"] }
```

### Basic Example

```rust
use perl_position_tracking::{ByteSpan, LineStartsCache};

let source = "line 1\nline 2\nline 3";
let cache = LineStartsCache::new(source);

// Create a span covering "line 2"
let span = ByteSpan::new(7, 13);
assert_eq!(span.slice(source), "line 2");

// Convert to line/column for LSP
let (line, col) = cache.offset_to_position(source, span.start);
assert_eq!(line, 1); // 0-indexed
assert_eq!(col, 0);
```

### UTF-16 Conversion

```rust
use perl_position_tracking::{offset_to_utf16_line_col, utf16_line_col_to_offset};

let source = "Hello 🦀 World";

// Convert byte offset to UTF-16 line/column
let (line, col) = offset_to_utf16_line_col(source, 9);

// Convert back to byte offset
let offset = utf16_line_col_to_offset(source, line, col);
```

## Architecture

This crate is used throughout the Perl LSP project:

- **perl-parser**: Uses `ByteSpan` for AST node positions
- **perl-lsp**: Converts positions for LSP protocol communication
- **perl-dap**: Uses position tracking for debugger locations

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

See the main [perl-lsp repository](https://github.com/perlwasm/perl-lsp) for contribution guidelines.
