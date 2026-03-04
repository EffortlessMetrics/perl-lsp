# perl-lsp-uri

Small SRP utility crate for panic-free `lsp_types::Uri` parsing.

## What it provides

- `parse_uri`: parse a string into `lsp_types::Uri` and fall back to a valid sentinel URI.
- `fallback_uri`: produce the stable fallback URI used by the parser.

This crate is intended for internal workspace use by LSP/DAP-facing crates that need robust URI parsing without panics.
