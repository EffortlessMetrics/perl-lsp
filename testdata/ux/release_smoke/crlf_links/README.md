# CRLF Links

## Scenario

A Windows user opens a CRLF Perl file that contains local quoted-file links and
module references. Document-link ranges should be computed from real line
offsets, not synthetic LF-only offsets.

## Files

- `main.pl` - CRLF source file with module and quoted local-file references.
- `lib/Smoke/CRLF.pm` - local module target.
- `notes/todo.txt` - quoted local-file target.

## Smoke Requests

```text
initialize
initialized
textDocument/didOpen main.pl
textDocument/documentLink main.pl
textDocument/definition on Smoke::CRLF
shutdown
```

## Expected Behavior

- Document links point at the exact quoted `notes/todo.txt` range.
- Module navigation resolves `Smoke::CRLF` to `lib/Smoke/CRLF.pm`.
- CRLF line endings do not shift links or diagnostics to the wrong character.

## Non-Goals

This fixture does not claim support for arbitrary string interpolation in file
paths.
