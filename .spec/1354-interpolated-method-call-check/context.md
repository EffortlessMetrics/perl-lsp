# Context: #1354 — Parser: Interpolated string delimiter check incorrectly flags method calls

## Problem

The parser's `find_unclosed_interpolation_delimiter` function in `primary.rs` contains overly aggressive logic that checks for balanced parentheses after method calls (e.g., `$obj->method()`) inside double-quoted strings. In Perl, method calls are **not** interpolated in strings—only scalar variables, array dereferences (`$arr->[idx]`), and hash dereferences (`$obj->{key}`) are interpolated. The parser's attempt to 'balance' parentheses in method calls leads to false `Unclosed ( delimiter in interpolated string` errors.

**Real-world impact:** Line 785 of DBI.pm in the CPAN corpus fails to parse:
```perl
$class->trace_msg("    -> $class->install_driver($driver"
        .") for $^O\n");
```

The string literal contains `$class->install_driver($driver` without a closing `)`, but this should not be flagged as an interpolation error because `->install_driver()` is literal text (not interpolated).

## Why this approach

**Root cause:** The function checks for two patterns that should never trigger:
1. `->` followed directly by `(` (lines 113–120)
2. `->` followed by an identifier and then `(` (lines 122–134)

Per Perl's interpolation rules, only these patterns are interpolated:
- `$identifier` (scalar)
- `$identifier->{key}` (hash dereference)
- `$identifier->[index]` (array dereference)

Method calls like `$identifier->methodname(...)` are **never** interpolated; the entire `->methodname(...)` part is literal text.

**Solution:** Delete the two problematic code blocks. Keep the checks for `->{}` and `->[]`, which are correct.

**Why not alternative approaches:**
- We could mark method calls differently to avoid the check, but that adds complexity.
- We could special-case method calls to skip balance checking, but that's what the deletion achieves directly.
- Removal is the cleanest and most maintainable solution (less code = fewer bugs).

## Alternatives rejected

- **Whitelist method calls explicitly**: Add a flag or marker to distinguish method calls from other `->` patterns. **Rejected** because the simplest fix is to remove the incorrect check entirely; method calls don't need special handling—they simply don't trigger interpolation at all.
- **Add heuristic for 'safe' method names**: Check if the identifier after `->` looks like a method (e.g., starts with lowercase, not `[` or `{`). **Rejected** because this adds complexity and fragility. The real issue is that Perl doesn't interpolate these at all, so any balancing check is wrong.
- **Document as a known limitation**: Mark the false positives as expected behavior. **Rejected** because the behavior is simply incorrect per Perl semantics and confuses users.

## Prior art / duplicates

**Perl interpolation rules** (perlop, "Quote and Quote-like Operators"):
> Within double-quoted strings, only `$scalar`, `@array`, `%hash`, and the escape sequences listed above are interpolated. All other constructs are left as-is.
> In particular, method calls are not interpolated. `"$obj->method()"` interpolates `$obj` and leaves `->method()` as literal text.

This is authoritative Perl behavior. The fix aligns the parser with this spec.

No existing implementation in perl-lsp correctly handles this case. The CPAN corpus failure on DBI.pm is direct evidence.

## Links

- **Issue:** #1354 — Parser: Interpolated string delimiter check incorrectly flags method calls
- **PARSER_CONTRACTS.md:** §Interpolation-Delimiters, §Error-Recovery
- **perlop (Perl 5 documentation):** "Quote and Quote-like Operators" section, interpolation rules
- **Related incidents:** Similar false-positive patterns may exist in regex and heredoc handling; recommend follow-up audit (separate issue).
