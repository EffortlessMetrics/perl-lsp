# perl-regex

Regex validation and safety checks for Perl patterns.

## Scope

- Validates regex syntax with offset-aware errors.
- Detects risky constructs (embedded code execution patterns).
- Applies heuristics for catastrophic-backtracking risk detection.

## Public Surface

- `RegexValidator`.
- `RegexError`.

## Workspace Role

Internal parser/security helper crate used by parse-time validation logic.

## License

MIT OR Apache-2.0.
