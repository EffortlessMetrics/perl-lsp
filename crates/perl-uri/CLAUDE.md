# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-uri` is a **Tier 1 leaf crate** providing URI ↔ filesystem path conversion utilities.

**Purpose**: URI ↔ filesystem path conversion and normalization utilities for Perl LSP — handles file:// URLs and platform-specific paths.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-uri              # Build this crate
cargo test -p perl-uri               # Run tests
cargo clippy -p perl-uri             # Lint
cargo doc -p perl-uri --open         # View documentation
```

## Architecture

### Dependencies

- `url` - URL parsing and manipulation

### Key Functions

| Function | Purpose |
|----------|---------|
| `uri_to_path` | Convert `file://` URI to filesystem path |
| `path_to_uri` | Convert filesystem path to `file://` URI |
| `normalize_uri` | Normalize URI for consistent comparison |

### Platform Handling

```rust
// Unix
// file:///home/user/code/script.pl → /home/user/code/script.pl

// Windows
// file:///C:/Users/code/script.pl → C:\Users\code\script.pl
// file:///c%3A/Users/code/script.pl → C:\Users\code\script.pl (encoded colon)
```

### URI Normalization

Normalization ensures consistent URI comparison:

```rust
use perl_uri::normalize_uri;

// These should all normalize to the same URI:
// file:///path/to/file.pl
// file:///path/to/../to/file.pl
// file:///path/to/./file.pl
```

## Usage

```rust
use perl_uri::{uri_to_path, path_to_uri};
use std::path::Path;

// URI to path
let uri = "file:///home/user/script.pl";
let path = uri_to_path(uri)?;
assert_eq!(path, Path::new("/home/user/script.pl"));

// Path to URI
let path = Path::new("/home/user/script.pl");
let uri = path_to_uri(path)?;
assert_eq!(uri, "file:///home/user/script.pl");
```

## Important Notes

- Always use these functions for URI/path conversion (not manual string manipulation)
- Handle percent-encoding of special characters
- Windows paths require special handling for drive letters
- Test on both Unix and Windows platforms
