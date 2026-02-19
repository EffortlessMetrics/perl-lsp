# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-uri` is a **Tier 1 leaf crate** providing URI-to-filesystem-path conversion and normalization utilities for the Perl LSP ecosystem.

**Version**: 0.9.1

## Commands

```bash
cargo build -p perl-uri              # Build this crate
cargo test -p perl-uri               # Run tests
cargo clippy -p perl-uri             # Lint
cargo doc -p perl-uri --open         # View documentation
```

## Architecture

### Dependencies

- `url` (external) -- URL parsing and manipulation
- `perl-tdd-support` (dev) -- test assertion helpers (`must`, `must_some`)
- `tempfile` (dev) -- temporary file utilities

### Key Functions

| Function | Platform | Purpose |
|----------|----------|---------|
| `uri_to_fs_path` | non-wasm | Convert `file://` URI to `PathBuf` via `url::Url::to_file_path` |
| `fs_path_to_uri` | non-wasm | Convert filesystem path to `file://` URI string; resolves relative paths |
| `normalize_uri` | non-wasm | Normalize a URI or bare path to a consistent `file://` form |
| `uri_key` | all | Normalize URI for map lookups (lowercases Windows drive letters) |
| `is_file_uri` | all | Check if URI uses `file://` scheme |
| `is_special_scheme` | all | Check if URI uses a non-file scheme (`untitled:`, `git:`, `vscode-notebook:`, `vscode-vfs:`) |
| `uri_extension` | all | Extract file extension from a URI string |

### Platform Handling

Functions marked *non-wasm* are gated with `#[cfg(not(target_arch = "wasm32"))]`. A minimal wasm32 version of `normalize_uri` exists that performs parse-only normalization without filesystem access.

### Windows Drive Letter Normalization

`uri_key` normalizes uppercase drive letters so that `file:///C:/foo` and `file:///c:/foo` resolve to the same lookup key.

## Usage

```rust
use perl_uri::{uri_to_fs_path, fs_path_to_uri, normalize_uri, uri_key};

// URI to path
let path = uri_to_fs_path("file:///tmp/test.pl"); // Some(PathBuf)

// Path to URI
let uri = fs_path_to_uri("/tmp/test.pl"); // Ok("file:///tmp/test.pl")

// Normalize arbitrary input
let uri = normalize_uri("file:///tmp/test.pl"); // "file:///tmp/test.pl"

// Consistent lookup key
let key = uri_key("file:///C:/Users/test.pl"); // "file:///c:/Users/test.pl"
```

## Important Notes

- Always use these functions for URI/path conversion instead of manual string manipulation
- `fs_path_to_uri` resolves relative paths via `std::env::current_dir` before conversion
- Percent-encoding/decoding is handled automatically by the `url` crate
- Tests use `perl_tdd_support::must` / `must_some` instead of `unwrap()` / `expect()`
- The `#[allow(clippy::unwrap_used, clippy::expect_used)]` attribute is applied only to the test module
