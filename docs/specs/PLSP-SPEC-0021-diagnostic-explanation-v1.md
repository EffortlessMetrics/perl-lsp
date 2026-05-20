# PLSP-SPEC-0021: Diagnostic explanation v1

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0012](PLSP-SPEC-0012-user-facing-trust-surfaces.md)
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0016](PLSP-SPEC-0016-provider-decision-receipt-v1.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: diagnostics, provider decision explanations, support tiers,
Real Perl Editor Trust dashboard
Schema: [diagnostic_explanation.v1.schema.json](../../schemas/diagnostic_explanation.v1.schema.json)

## Current Implementation Status

Diagnostic explanation payloads are attached to provider decision receipts for
`textDocument/diagnostic` and `workspace/diagnostic`. The payload is
explanation-only: it describes returned diagnostics, trust boundaries, and
module-resolution context without changing suppression, severity, resolver
behavior, workspace scanning, support tiers, or provider promotion state.

## Contract

Every `diagnostic_explanation.v1` payload must include:

- `schema_version = "diagnostic_explanation.v1"`
- `surface = "diagnostics"`
- `decision = "explanation_only"`
- `provider_action`
- `fact_source`
- `confidence`
- `freshness`
- diagnostic counts and truncation state
- a list of diagnostic explanation items
- dynamic-boundary summary state
- a claim boundary that keeps the payload explanation-only

Each diagnostic explanation item must include:

- trust boundary
- severity label
- summary
- reason
- why the diagnostic fired
- why the diagnostic was not suppressed

PL701 missing-module explanations must preserve module-resolution context:

- requested module
- expected relative module path
- reported `@INC` path classes
- whether effective include paths were reported
- whether workspace include paths were labeled
- PERL5LIB policy label
- whether searched `@INC` context was reported

## Valid PR Shapes

Valid PRs under this spec include:

- adding fields to `diagnostic_explanation.v1` without removing existing fields
- adding focused explanation tests for one diagnostic code or trust boundary
- adding schema snapshots or validators
- improving user text while preserving canonical fields
- adding redaction or path-classification proof for diagnostic explanation
  payloads

## Invalid PR Shapes

Invalid PRs include:

- changing diagnostic suppression from an explanation-only PR
- changing diagnostic severity from an explanation-only PR
- changing module resolution or workspace scanning from an explanation-only PR
- probing Perl, running `perldoc`, or launching DAP to enrich an explanation
- promoting diagnostic support tiers from explanation presentation alone
- treating low-confidence, stale, generated/no-source, dynamic, or ambiguous
  evidence as exact suppression proof

## Acceptance

A diagnostic explanation PR satisfies this spec when:

- live diagnostic explanation payloads conform to the v1 schema-required fields
- provider decision copyable receipts preserve the diagnostic explanation payload
- PL701 explanations expose missing-module lookup context
- claim boundaries explicitly state no suppression, severity, or support-tier
  promotion
- diagnostics support remains `partial-live-with-fallback`

## Proof Commands

```bash
cargo test -p perl-lsp-rs live_diagnostic_request_attaches_explainable_payload --lib --profile agent --locked -- --nocapture --test-threads=1
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
cargo xtask ci-hygiene check-doc-paths docs/specs
powershell -NoProfile -Command "Get-Content schemas/diagnostic_explanation.v1.schema.json -Raw | ConvertFrom-Json | Out-Null"
git diff --check
```

## Non-goals

- No diagnostic suppression change.
- No diagnostic severity change.
- No resolver behavior change.
- No workspace scanning change.
- No Perl, `perldoc`, DAP, or subprocess probing.
- No provider promotion or support-tier promotion.
- No rename or safe-delete authorization.

## Claim Boundaries

This spec may claim that `perl-lsp` can explain returned diagnostics and their
trust boundaries through a versioned payload. It may not claim broad diagnostic
correctness, new suppression authority, or compiler-backed diagnostic cutover.
