# perl-uri

URI and filesystem-path normalization for Perl tooling.

Use this crate when a caller may hand you either a `file://` URI or a raw
filesystem path and you need a stable representation for caches, lookups, or
workspace edges.

## Where it fits

`perl-uri` sits at the protocol boundary. It is used by LSP and DAP code before
files hit the parser, workspace index, or other path-sensitive layers.

## Key entry points

- `uri_to_fs_path(uri)` - convert `file://` URIs to filesystem paths
- `fs_path_to_uri(path)` - convert filesystem paths to `file://` URIs
- `normalize_uri(uri)` - normalize raw paths, malformed file URIs, and valid URIs
- `uri_key(uri)` - produce a lookup key with Windows drive-letter normalization
- `is_file_uri(uri)` and `is_special_scheme(uri)` - classify URIs
- `uri_extension(uri)` - extract the extension from a URI string

## Example

```rust
use perl_uri::{normalize_uri, uri_key};

#[cfg(windows)]
assert_eq!(normalize_uri(r"C:\workspace\script.pl"), "file:///c:/workspace/script.pl");
assert_eq!(uri_key("file:///C:/Users/test.pl"), "file:///c:/Users/test.pl");
```

## Typical use

Use `perl-uri` when you need consistent cache keys or filesystem lookups
without making callers care whether they passed a URI or a path. On `wasm32`,
only the URI-parsing helpers are available.
