# perl-dap-security-patterns

Shared security pattern catalog for Perl DAP safe expression evaluation.

This crate contains:

- `DANGEROUS_OPERATIONS`: blocked Perl built-ins for safe eval mode
- `ASSIGNMENT_OPERATORS`: mutation-indicating assignment operators
- `DANGEROUS_OPS_RE`: compiled regex for blocked operation detection
- `REGEX_MUTATION_RE`: compiled regex for `s///`, `tr///`, `y///` detection

It is used by `perl-dap-eval` to keep policy data and pattern compilation separate from validation flow logic.
