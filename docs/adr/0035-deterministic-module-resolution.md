# ADR-0035: Deterministic Module Resolution with Canonicalized Names

- **Status**: Accepted
- **Date**: 2026-03-18
- **Related**: [ADR-0008](0008-microcrate-architecture.md), [ADR-0017](0017-workspace-exclusion-strategy.md)

## Context

Perl module resolution appears in multiple workflows across the codebase, including module goto-definition/navigation, `documentLink/resolve` for module references, and workspace-aware module lookup inside the LSP runtime. The implementation is intentionally split across microcrates rather than being embedded in a single LSP-only helper:

- `perl-module-name` normalizes canonical `::` names and legacy `'` separators.
- `perl-module-path` converts between module names and `.pm` file paths.
- `perl-module-resolution-path` resolves filesystem candidates under a workspace root.
- `perl-module-resolution-uri` resolves `file://` URIs across open documents, workspace folders, and optional system `@INC` paths.
- `perl-module-resolution` re-exports the path and URI entry points as the integration surface.

The code also encodes several concrete behavioral decisions that were not previously captured together in a single ADR:

1. **Legacy separator compatibility**: `Foo'Bar` is normalized to `Foo::Bar` before path conversion.
2. **Deterministic precedence**: URI resolution prefers already-open documents, then workspace folders + include paths, then optional system `@INC`.
3. **Workspace safety**: include-path candidates are validated against the workspace root and traversal attempts are ignored.
4. **Pragmatic fallback**: path resolution returns `root/lib/<module>.pm` even when the file is not yet present, so higher layers can still propose or stage a target path.
5. **Microcrate composition**: name normalization, path conversion, path lookup, and URI lookup evolve independently but are combined through a single facade crate.

Without an ADR, contributors must reconstruct these rules by reading several crates and tests.

## Decision Drivers

- Preserve the repository's microcrate architecture.
- Keep module-name compatibility for both canonical `::` and legacy `'` syntax.
- Prefer editor-visible and workspace-local sources over global interpreter state.
- Prevent include-path traversal from escaping the workspace root.
- Provide deterministic results even when a target module file has not been created yet.

## Decision

We standardize Perl module resolution around the following architecture and policy.

### 1. Canonicalize names first

All module-resolution entry points treat `::` as the canonical package separator. Legacy `'` separators remain supported at the API edge, but they are normalized before path conversion or URI/path resolution.

### 2. Keep resolution decomposed into microcrates

We preserve the current microcrate split:

- `perl-module-name`: separator normalization and canonical/legacy projections.
- `perl-module-path`: string-level conversion between module names and paths.
- `perl-module-resolution-path`: workspace-rooted path lookup.
- `perl-module-resolution-uri`: URI-first search with timeout budgeting.
- `perl-module-resolution`: facade crate that re-exports the public integration API.

This matches the repository-wide microcrate architecture while keeping each crate testable in isolation.

### 3. Resolve URIs with explicit precedence

Module URI resolution follows this fixed order:

1. **Open document URIs** matching the relative module path.
2. **Workspace folders + include paths** after path-security validation.
3. **System `@INC` paths** only when explicitly enabled.

This order favors editor-visible buffers first, then workspace-local files, and only then global interpreter state.

### 4. Enforce workspace-bound path safety

Workspace-rooted path candidates produced from include paths must pass path validation. Traversal-style include paths are skipped rather than producing an error or escaping the workspace.

### 5. Return stable fallback paths for unresolved local modules

Filesystem resolution returns `root/lib/<module>.pm` as the fallback candidate when no safe include-path hit exists. This is a deliberate design choice for IDE workflows that need a deterministic target path even before a module file exists.

### 6. Bound URI resolution with timeouts

URI resolution remains timeout-aware and may return `TimedOut` rather than blocking indefinitely while scanning workspace folders or system include paths.

## Alternatives Considered

### Single monolithic resolver crate

Rejected. The codebase already separates name normalization, path conversion, filesystem lookup, and URI lookup into dedicated crates. Keeping that split matches the existing architecture and makes focused tests easier to maintain.

### Search system `@INC` before workspace folders

Rejected. Workspace-local modules and already-open editor documents are more relevant to navigation and document-link resolution than interpreter-global installations.

### Return `None` when a local module is missing on disk

Rejected. Higher layers benefit from a stable fallback path for actions such as goto-definition heuristics and rename destination planning.

## Consequences

### Positive

- **Consistent behavior across features**: navigation, document-link resolution, and workspace-facing module lookup all share the same normalization and precedence model.
- **Backward compatibility**: legacy `'` module references remain supported without forcing downstream crates to carry multiple naming schemes internally.
- **Security hardening**: path traversal via include paths is rejected by construction.
- **Predictable UX**: editor buffers win over disk, and unresolved modules still map to a deterministic `lib/` destination.
- **Testability**: each concern is covered by focused unit tests at the crate boundary.

### Negative

- **Extra indirection**: understanding module resolution requires following several small crates instead of one larger implementation.
- **Fallback can over-approximate**: `root/lib/<module>.pm` may be returned for a module that does not actually exist yet.
- **Open-document precedence is suffix-based**: URI matching is intentionally simple and depends on relative path suffix matches.

### Neutral / Follow-up

- Future ADRs may document more specialized module-resolution topics, such as broader static-versus-dynamic resolution boundaries or additional compatibility rules around nonstandard loader behavior.

## Implementation Notes

This ADR describes behavior already present in the codebase:

- Canonical and legacy separator handling lives in `perl-module-name` and `perl-module-path`.
- Workspace-safe path lookup lives in `perl-module-resolution-path`.
- URI precedence, timeout handling, and optional `@INC` scanning live in `perl-module-resolution-uri`.
- Integration tests in `crates/perl-module-resolution/tests/` encode the precedence and fallback behavior.
- This ADR records the existing implementation; it does not introduce a new module-resolution mechanism.
