# perl-uri

URI ↔ filesystem path conversion and normalization utilities for the Perl LSP ecosystem.

## Features

- **URI to path conversion**: Convert `file://` URIs to filesystem paths with proper percent-decoding
- **Path to URI conversion**: Convert filesystem paths to `file://` URIs with proper percent-encoding
- **URI normalization**: Normalize URIs to consistent forms for reliable lookups
- **Windows support**: Handle drive letter normalization (`C:` → `c:`)
- **Special scheme handling**: Preserve special URIs like `untitled:`, `git:`, etc.

## Usage

```rust
use perl_uri::{uri_to_fs_path, fs_path_to_uri, uri_key};

// Convert a URI to a path
let path = uri_to_fs_path("file:///tmp/test.pl");
assert!(path.is_some());

// Convert a path to a URI
let uri = fs_path_to_uri("/tmp/test.pl").unwrap();
assert!(uri.starts_with("file:///"));

// Normalize a URI key for consistent lookups
let key = uri_key("file:///C:/Users/test.pl");
assert_eq!(key, "file:///c:/Users/test.pl");  // Lowercase drive letter
```

## Platform Support

Most functions are not available on `wasm32` targets since they require filesystem access.
The `uri_key` and `is_*` helper functions are available on all platforms.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
