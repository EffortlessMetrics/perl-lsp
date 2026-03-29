# perl-uri

URI and filesystem-path conversion utilities for Perl editor tooling.

## When to use this crate

Use `perl-uri` when you need a small utility crate for:

- converting between `file://` URIs and local filesystem paths
- normalizing Windows and Unix file references into stable URI keys
- handling raw absolute paths that may arrive from editors or test harnesses

It is useful outside the full `perl-lsp` runtime when you only need file-URI
logic.

## Public API

| Function | Purpose |
|----------|---------|
| `uri_to_fs_path` | Convert a `file://` URI to a `PathBuf` (non-wasm only) |
| `fs_path_to_uri` | Convert a filesystem path to a `file://` URI string (non-wasm only) |
| `normalize_uri` | Normalize a URI (or bare path) to a consistent `file://` form |
| `uri_key` | Normalize a URI for use as a lookup key (lowercases Windows drive letters) |
| `is_file_uri` | Check whether a URI uses the `file://` scheme |
| `is_special_scheme` | Check whether a URI uses a non-file scheme (`untitled:`, `git:`, etc.) |
| `uri_extension` | Extract the file extension from a URI string |

## Usage

```rust
use perl_uri::{uri_to_fs_path, fs_path_to_uri, uri_key};

let path = uri_to_fs_path("file:///tmp/test.pl");    // Some(PathBuf)
let uri  = fs_path_to_uri("/tmp/test.pl").unwrap();   // "file:///tmp/test.pl"
let key  = uri_key("file:///C:/Users/test.pl");        // "file:///c:/Users/test.pl"
```

## Platform support

`uri_to_fs_path`, `fs_path_to_uri`, and `normalize_uri` require filesystem access and are
not available on `wasm32` targets. The remaining helpers work on all platforms.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
