# Spec: Fix SemVer Hygiene for Published Crates — work-e7f94205

## Feature / Behavior Description

Address `cargo semver-checks` findings from the v0.12.1 baseline across 3 published crates:

1. **`perl-diagnostics`**: Move `UnreachableCode` variant to end of `DiagnosticCode` enum (restoring v0.12.1 discriminant values) and add `#[non_exhaustive]`
2. **`perl-lsp-config`**: Add `#[non_exhaustive]` to `ServerConfig` and `AiCompletionConfig`
3. **`perl-semantic-analyzer`**: Add `#[non_exhaustive]` to `ClassModel`, `MethodInfo`, `Attribute`, and `MethodModifier`

## Acceptance Criteria

### AC1: `DiagnosticCode` enum discriminant restored

- `UnreachableCode` variant is located after `CriticSeverity5` (end of enum body)
- All 18 affected variants (`EvalErrorFlow` through `CriticSeverity5`) retain their v0.12.1 discriminant values after the reorder
- `as_str()` match arm for `UnreachableCode` returns `"PL406"` (already correct)
- `#[non_exhaustive]` attribute is present on `DiagnosticCode`
- `cargo semver-checks check-release -p perl-diagnostics --baseline-rev v0.12.1` reports no `enum_variant_added` or `enum_no_repr_variant_discriminant_changed` findings

### AC2: `perl-lsp-config` structs non-exhaustive

- `ServerConfig` (line 22) has `#[non_exhaustive]`
- `AiCompletionConfig` (line 120) has `#[non_exhaustive]`
- `cargo semver-checks check-release -p perl-lsp-config --baseline-rev v0.12.1` reports no `constructible_struct_adds_field` findings for these types

### AC3: `perl-semantic-analyzer` types non-exhaustive

- `ClassModel` (line 177) has `#[non_exhaustive]`
- `MethodInfo` (line 148) has `#[non_exhaustive]`
- `Attribute` (line 59) has `#[non_exhaustive]`
- `MethodModifier` (line 111) has `#[non_exhaustive]`
- `cargo semver-checks check-release -p perl-semantic-analyzer --baseline-rev v0.12.1` reports no `constructible_struct_adds_field` findings for these types

### AC4: Internal struct literal callers migrated

- All internal usages of `ServerConfig { ... }` and `AiCompletionConfig { ... }` have been converted to `::default()` or a builder pattern
- The `ClassModelBuilder` pattern is already used for `ClassModel` construction (no changes needed)
- Code compiles without errors after `#[non_exhaustive]` is added

### AC5: `perl-lsp` binary crate excluded

- `DocumentState` is NOT modified (binary crate `[[bin]]`, not a published library, has no external API surface per vision alignment)

### AC6: Clean workspace semver check (post-fix)

- `cargo semver-checks check-release --workspace --baseline-rev v0.12.1` runs clean (or per-crate re-run succeeds if bare-repo bug triggers)

## Non-Goals

- CI integration of `cargo semver-checks` gate (separate work item)
- Audit of 3 net-new crates (`perl-heredoc-anti-patterns`, `perl-lsp-ai-provider`, `perl-parser-bench`) — separate work item
- Fixing the cargo bare-repo bug in semver-checks itself

## Dependencies

- `cargo semver-checks` v0.45.0+ (v0.47.0 currently installed)
- Git baseline `v0.12.1` tag for comparison
- Internal `Default` impls on `ServerConfig`, `AiCompletionConfig` must exist before `#[non_exhaustive]` is added (they do — verified in plan review)
