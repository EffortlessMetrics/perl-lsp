# ADR/Spec Agent Findings — work-e7f94205

## What This ADR Decides

Adopt `#[non_exhaustive]` on 6 public types across 2 published crates and fix the `DiagnosticCode` discriminant bug by moving `UnreachableCode` to the end of the enum body. The decision splits the work into two conceptual parts: a bug-fix commit (discriminant restoration) and a hygiene commit (`#[non_exhaustive]` additions), both on the same branch.

## Key Decision

Add `#[non_exhaustive]` to `DiagnosticCode`, `ServerConfig`, `AiCompletionConfig`, `ClassModel`, `MethodInfo`, `Attribute`, and `MethodModifier`. Simultaneously, move `UnreachableCode` from mid-enum position to end of `DiagnosticCode` to restore v0.12.1 discriminant values. Drop `DocumentState` from scope (binary crate, no external API surface).

## Alternatives Considered

1. **Status quo (no `#[non_exhaustive]`)** — rejected: forces unnecessary major version bumps for minor field additions
2. **Fix only `DiagnosticCode` discriminant, skip hygiene** — rejected: defers the same problem
3. **Drop hygiene entirely, only fix discriminant ordering** — rejected: low-effort fixes with high long-term value

## Consequences

- Downstream consumers using `code as isize` on `DiagnosticCode` will get correct v0.12.1 discriminant values again
- Future minor-version field additions on covered types won't be SemVer-breaking
- Internal `ServerConfig`/`AiCompletionConfig` callers must migrate to `::default()` (both have `Default` impls)
- `perl-lsp` binary crate `DocumentState` excluded per vision alignment finding

## Acceptance Criteria

(from specs.md)
1. `DiagnosticCode` `UnreachableCode` at enum end, discriminants restored, `#[non_exhaustive]` present, semver check clean
2. `ServerConfig` and `AiCompletionConfig` have `#[non_exhaustive]`, semver check clean
3. `ClassModel`, `MethodInfo`, `Attribute`, `MethodModifier` have `#[non_exhaustive]`, semver check clean
4. Internal struct literal callers migrated to `::default()` or builder
5. `DocumentState` excluded (binary crate)
6. Workspace semver check clean after all fixes

## Friction

- **Crate name discrepancy**: initial plan referenced `perl-diagnostics-codes` but actual crate is `perl-diagnostics` (path `crates/perl-diagnostics/src/codes/mod.rs`). Required cross-checking Cargo.toml workspace members to resolve.
- **Version provenance unverified**: vision alignment agent flagged findings may span v0.12.2/v0.12.3/v0.12.4, not all v0.12.2. The hygiene scope is correct regardless, but baseline validity needs confirmation before merge.
- **Cargo bare-repo bug**: documented environment issue, not code risk.
