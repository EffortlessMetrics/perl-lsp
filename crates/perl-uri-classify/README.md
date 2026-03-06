# perl-uri-classify

Shared URI classification and key normalization helpers for the Perl LSP ecosystem.

## Provided helpers

- `uri_key` — normalizes URI keys for stable lookups (including Windows drive-letter normalization)
- `is_file_uri` — checks for `file://` scheme
- `is_special_scheme` — detects non-file/special URI schemes
- `uri_extension` — extracts file extension from URI-like strings
