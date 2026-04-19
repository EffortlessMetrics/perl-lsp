# ADR-0044: Fix `DiagnosticCode` Discriminant Bug and Adopt `#[non_exhaustive]` Hygiene

## Status

Proposed

## Context

`cargo semver-checks check-release --workspace --baseline-rev v0.12.1` (v0.45.0) flagged 5 breaking API changes across 4 crates. Detailed analysis reveals two distinct problem classes:

1. **Bug (real breaking change)**: The `UnreachableCode` variant was inserted at line 161 of `DiagnosticCode` — mid-enum, after `PrintfFormatMismatch` — rather than appended at the end. This shifted the discriminant values of 18 subsequent variants (`EvalErrorFlow` through `CriticSeverity5`). The string codes (`as_str()`) are correct; only raw `code as isize` casting is broken. This violates the workspace convention documented in `perl-diagnostics/CLAUDE.md`: "never renumber existing codes; new codes are appended at the end."

2. **Hygiene (prospective future breaks)**: `ServerConfig`, `AiCompletionConfig`, and `ClassModel` lack `#[non_exhaustive]`, meaning any minor-version addition of fields is a SemVer-breaking change. The workspace already has 2 instances of `#[non_exhaustive]` (`perl-lsp-config` line 576, `perl-workspace-index`), establishing precedent.

**Version provenance note**: The vision alignment agent identified that some findings (e.g., `ServerConfig.ai_completion`) may trace to v0.12.3 or later, not v0.12.2. The root cause — missing `#[non_exhaustive]` — is the same regardless of which release first introduced the issue.

**Out of scope**: `DocumentState` in `perl-lsp` (binary crate, no external library consumers) and the 3 net-new crates (separate work item).

## Decision

### Part A — Bug Fix (DiagnosticCode)

1. Move `UnreachableCode` variant from its current mid-enum position (after `PrintfFormatMismatch`, before `EvalErrorFlow`) to the end of the enum body, after `CriticSeverity5`. This restores v0.12.1 discriminant values with no other changes.
2. Add `#[non_exhaustive]` to `DiagnosticCode` to prevent future mid-enum insertions.

The `as_str()` match arm `DiagnosticCode::UnreachableCode => "PL406"` already correctly maps to `"PL406"` (end of PL400 range). All match arms for `as_str()`, `severity()`, `category()`, `tags()`, `documentation_url()`, and `parse_code()` use name-based patterns (`DiagnosticCode::VariantName =>`), not order-based patterns, so variant reordering is safe.

### Part B — Hygiene (#[non_exhaustive] on published library types)

Add `#[non_exhaustive]` to:

| Crate | Type | File |
|-------|------|------|
| `perl-lsp-config` | `ServerConfig` | `crates/perl-lsp-config/src/lib.rs` line 22 |
| `perl-lsp-config` | `AiCompletionConfig` | `crates/perl-lsp-config/src/lib.rs` line 120 |
| `perl-semantic-analyzer` | `ClassModel` | `crates/perl-semantic-analyzer/src/analysis/class_model.rs` line 177 |
| `perl-semantic-analyzer` | `MethodInfo` | `crates/perl-semantic-analyzer/src/analysis/class_model.rs` line 148 |
| `perl-semantic-analyzer` | `Attribute` | `crates/perl-semantic-analyzer/src/analysis/class_model.rs` line 59 |
| `perl-semantic-analyzer` | `MethodModifier` | `crates/perl-semantic-analyzer/src/analysis/class_model.rs` line 111 |

All listed types are part of public library APIs consumed by downstream crates. `ServerConfig` and `AiCompletionConfig` both have `Default` impls; callers must migrate to `::default()` construction.

## Consequences

### Benefits

- **`DiagnosticCode` discriminant bug is fixed**: downstream consumers doing `code as isize` arithmetic will see correct v0.12.1 values again
- **Enum insertion safety**: `#[non_exhaustive]` prevents future mid-enum insertions from being SemVer-breaking
- **Struct field addition safety**: `#[non_exhaustive]` on config structs allows future minor-version fields without major bumps
- **Consistent with existing workspace pattern**: 2 existing `#[non_exhaustive]` instances confirm this is an accepted convention

### Tradeoffs / Risks

1. **Enum reorder is a theoretical break for any code matching on variant position** — mitigated by all workspace match arms using name-based patterns
2. **Internal callers using struct literals for `ServerConfig`/`AiCompletionConfig`** must migrate to `::default()` — both structs already have `Default` impls, so this is a non-breaking refactor internally
3. **`perl-semantic-analyzer` `ClassModel` is built via `ClassModelBuilder`** — internal callers are already using the builder, not struct literals
4. **Version provenance remains unverified** — the vision alignment agent flagged that some findings may be from v0.12.3/v0.12.4. The ADR scope covers the hygiene fix regardless of which version first introduced the issue, but the semver baseline validity (crate renamed `perl-diagnostics-codes` → `perl-diagnostics` after Wave E) needs confirmation before merging
5. **CI bare-repo bug** — first `cargo semver-checks` run hit "did not expect repo at .git to be bare" on 8 crates; per-crate re-runs succeeded. This is an environment/tool issue, documented in friction log

## Alternatives Considered

### Alternative 1: No `#[non_exhaustive]` (status quo)

Continue without `#[non_exhaustive]` on any types. Every minor-version field addition or enum variant insertion becomes a SemVer-major event. This forces unnecessary major version bumps and creates upgrade friction for downstream consumers.

**Rejected**: Goes against established workspace precedent and the documented intent of the issue.

### Alternative 2: Fix only `DiagnosticCode`, leave other types as-is

Address the real bug but skip the hygiene `#[non_exhaustive]` additions. Future minor-version field additions on `ServerConfig`, `AiCompletionConfig`, and `ClassModel` will trigger the same semver-checks failures at the next release.

**Rejected**: Defers the same problem to the next release cycle without any additional effort.

### Alternative 3: Drop all hygiene changes, only fix discriminant ordering

Only move `UnreachableCode` to the end of the enum without adding any `#[non_exhaustive]` annotations. This fixes the immediate bug but leaves all struct types vulnerable to future SemVer breaks.

**Rejected**: The hygiene fixes are low-effort, high-value, and already identified by the semver audit.

## References

- Research analysis: `/home/hermes/.hermes/state/conveyor/work-e7f94205/findings/research-agent-findings.md`
- Plan review: `/home/hermes/.hermes/state/conveyor/work-e7f94205/findings/plan-reviewer-findings.md`
- Vision alignment: prior agent `vision_alignment_comment` artifact
- Cargo.toml workspace members confirm actual crate name is `perl-diagnostics` (not `perl-diagnostics-codes`)
