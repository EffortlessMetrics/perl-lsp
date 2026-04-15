# ADR/Spec Findings — work-31c90e78

## What This ADR Decides

This ADR documents the architectural decision to add WalkDir-based filesystem scanning directly in `perl-lsp-completion` for module enumeration, rather than extending the `perl-module-resolution` microcrate family. The decision was needed because `perl-module-resolution` is designed for module resolution (path lookup), not module enumeration (directory scanning for all matching .pm files).

## Key Decision

We chose to add a separate WalkDir scanning implementation in `perl-lsp-completion` with its own limits (`MAX_SCAN_DEPTH`, `MAX_SCAN_ENTRIES`, `SCAN_TIMEOUT_MS`) rather than extending `perl-module-resolution`. This preserves separation of concerns between resolution (finding a known module) and enumeration (finding all modules matching a prefix), allows independent evolution, and enabled faster implementation.

## Alternatives Considered

1. **Extend perl-module-resolution with enumeration capability** — Rejected because different use case (enumeration vs resolution), different timeout semantics, would require API changes to a core dependency

2. **Use cached include path scanning** — Rejected because ADR decision deferred caching to v0.13.0; adding caching would delay the fix

3. **Only scan workspace index** — Rejected because does not address the reported issue; users with external dependencies would not see them in completion

## Consequences

**Benefits:**
- Users see complete module suggestions from configured include paths
- 30ms timeout and entry limits keep completion responsive
- Cancellation support allows graceful中断

**Tradeoffs:**
- Off-by-one bug in `MAX_SCAN_DEPTH` requires fix (should be 6, not 5)
- Microcrate drift: a second WalkDir scanning implementation exists separately from `perl-module-resolution`
- No caching: scanning happens on every completion request (acceptable per ADR deferral)

## Acceptance Criteria

From specs.md:
1. Include paths are scanned for module completions
2. System @INC paths are scanned when enabled
3. Prefix filtering works correctly
4. Depth limit is enforced (currently broken due to off-by-one)
5. Results are deduplicated across paths
6. Timeout and entry limits prevent hangs
7. Cancellation returns partial results

## What's in Scope

- `.perl-lsp.toml` `includePaths` configuration
- `PERL5LIB` and system `@INC` scanning
- `use` and `require` statement completion triggers
- Prefix filtering, depth limiting, timeout, cancellation
- WASM32 graceful exclusion

## What's Out of Scope

- Caching of scanned modules (deferred to v0.13.0)
- Module resolution for goto-definition (handled by `perl-module-resolution`)
- Fuzzy/partial module name matching

## Specs Summary

The feature adds @INC path scanning to module completion for `use` and `require` statements. After implementation, users will see external modules from configured `includePaths` and system `@INC` alongside workspace modules, properly deduplicated and filtered by prefix, with performance guards (30ms timeout, 10,000 entry limit) and cancellation support.

## Known Bugs Requiring Fixes

1. **Bug 1**: `property_prefix_filtering_exact` test has incorrect expectation — `Alphabet::Module` correctly appears for prefix `Alpha` because `Alphabet.starts_with("Alpha")` is true

2. **Bug 2**: `MAX_SCAN_DEPTH = 5` should be `6` to match documented behavior and test expectations — WalkDir's `max_depth(5)` excludes files at depth 6
