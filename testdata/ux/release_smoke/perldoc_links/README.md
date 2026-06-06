# Perldoc Links

## Scenario

A user opens code that uses common Perl modules and wants useful docs from the
editor. The server should expose hover or document-link behavior where it has a
supported docs path, and remain quiet where docs cannot be resolved.

## Files

- `docs.pl` - uses `File::Spec`, `JSON::PP`, and a local quoted README path.

## Smoke Requests

```text
initialize
initialized
textDocument/didOpen docs.pl
textDocument/hover on File::Spec
textDocument/hover on JSON::PP
textDocument/documentLink docs.pl
shutdown
```

## Expected Behavior

- Hover or perldoc links for known modules are useful when available.
- Missing perldoc support returns an empty or unsupported response without noisy
  warnings.
- The quoted `README.md` local path is a document-link candidate.
- Diagnostics should not report module-missing noise for common core modules.

## Non-Goals

This fixture does not require network documentation, CPAN metadata lookup, or
HTML rendering.
