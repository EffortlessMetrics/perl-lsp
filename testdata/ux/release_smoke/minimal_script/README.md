# Minimal Script

## Scenario

A user opens a single Perl script with no project metadata. The server should
initialize quietly, parse the file, publish only useful diagnostics, and keep
basic editor features responsive.

## Files

- `bin/hello.pl` - ordinary script with strict, warnings, lexical variables,
  and one local subroutine.

## Smoke Requests

```text
initialize
initialized
textDocument/didOpen bin/hello.pl
textDocument/documentSymbol bin/hello.pl
textDocument/completion bin/hello.pl at "$me"
textDocument/codeAction bin/hello.pl over an empty diagnostic set
shutdown
```

## Expected Behavior

- Startup produces no panic and no warning for normal single-file projects.
- Diagnostics are empty or informational only.
- Document symbols include `greet`.
- Completion near `$message` may include visible lexicals; if uncertain, it may
  return an empty list rather than noisy guesses.
- Code actions over no diagnostics return an empty result.

## Non-Goals

This fixture does not claim cross-file navigation, perldoc, or dependency
resolution.
