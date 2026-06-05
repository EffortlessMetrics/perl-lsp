# Lib Project

## Scenario

A user opens a common Perl project with executable scripts under `bin/` and
modules under `lib/`. The editor should use the effective include path from the
source file and resolve the workspace module consistently.

## Files

- `bin/app.pl` - script that adds `lib` to `@INC` and uses `Smoke::Greeter`.
- `lib/Smoke/Greeter.pm` - local module with a constructor and method.

## Smoke Requests

```text
initialize
initialized
textDocument/didOpen bin/app.pl
textDocument/definition on Smoke::Greeter
textDocument/definition on greet
textDocument/completion after "$greeter->"
workspace/symbol query "Greeter"
textDocument/documentLink bin/app.pl
shutdown
```

## Expected Behavior

- `Smoke::Greeter` resolves to `lib/Smoke/Greeter.pm`.
- Goto-definition and workspace symbol lookup agree on the module location.
- Completion after `$greeter->` is quiet if method confidence is too low, but
  should not offer unrelated workspace noise.
- Document links include useful local/module links when supported.

## Non-Goals

This fixture does not require runtime execution or external CPAN modules.
