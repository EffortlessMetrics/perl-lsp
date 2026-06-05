# Local Lib Project

## Scenario

A user opens a project that keeps installed dependencies under
`local/lib/perl5`. The editor should honor that explicit include path without
treating the entire workspace root as an implicit module wildcard.

## Files

- `script/report.pl` - script that adds `local/lib/perl5` to `@INC`.
- `local/lib/perl5/Local/Report.pm` - local dependency-style module.

## Smoke Requests

```text
initialize
initialized
textDocument/didOpen script/report.pl
textDocument/definition on Local::Report
textDocument/completion after "$report->"
workspace/symbol query "Report"
textDocument/documentLink script/report.pl
shutdown
```

## Expected Behavior

- `Local::Report` resolves to `local/lib/perl5/Local/Report.pm`.
- Symbols from `Local::Report` are visible through the same effective include
  context used by definition and completion.
- No diagnostics are emitted for the explicit local dependency path.
- Unsupported or low-confidence completion stays quiet.

## Non-Goals

This fixture does not claim support for dynamic `PERL5LIB` mutation or CPAN
installation.
