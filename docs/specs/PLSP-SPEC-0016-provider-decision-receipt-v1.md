# PLSP-SPEC-0016: Provider decision receipt v1

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0012](PLSP-SPEC-0012-user-facing-trust-surfaces.md)
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: provider confidence matrix, provider promotion ledger, support
tiers, UX capability dashboard, Real Perl Editor Trust dashboard

## Current Implementation Status

`perl-lsp` exposes provider decision explanations through
`perl.explainProviderDecision`. The current Rust model serializes
`schema_version = "provider_decision.v1"` and attaches optional copyable bug
report payloads with
`schema_version = "provider_decision_bug_report.v1"`.

This spec locks the v1 receipt contract. It is intentionally additive: future
provider surfaces may add fields, but they must not remove or reinterpret the
core decision fields without a new schema version.

The JSON schema for the receipt lives at
[schemas/provider_decision.v1.schema.json](../../schemas/provider_decision.v1.schema.json).

## Contract

Provider decision receipts explain why a provider acted, used fallback, blocked
an unsafe action, or recorded explanation-only evidence. The receipt is the
canonical decision payload. `user_message` is presentation text and must not be
the source of truth for provider behavior.

Every v1 receipt must expose:

- `schema_version`
- `provider`
- `decision`
- `reason`
- `fact_source`
- `confidence`
- `freshness`
- `dynamic_boundary`
- `fallback`

Receipts should expose these fields when the evidence exists:

- `receipt_id`
- `scenario`
- `request_receipt`
- `user_message`
- `copyable_payload`
- `source_backed`
- `blocker`
- `claim_boundary`

Provider-local `request_receipt` objects may keep provider-specific fields, but
they must preserve the normalized v1 fields when provider decision code
normalizes them.

## Provider Surfaces

The v1 receipt surface vocabulary includes:

```text
completion
goto_definition
type_definition
references
hover
diagnostics
rename
safe_delete
workspace_symbols
document_symbols
semantic_tokens
module_resolution
dap_module_paths
perl_subprocess
workspace_trust_report
unknown
```

`workspace_trust_report` may use its own report schema for the report body. If
it participates in a provider decision explanation, the receipt must preserve
the same report-only boundary: no probing, no subprocess launch, no DAP start,
and no support-tier promotion from presentation alone.

## Decision Vocabulary

The promotion ledger uses the policy vocabulary:

```text
promote
fallback
block
defer
```

The v1 wire receipt uses the current provider-decision vocabulary:

```text
acted
fallback
blocked
shadowed
```

These map as follows:

| Ledger state | Receipt decision | Meaning |
|---|---|---|
| `promote` | `acted` | The provider returned scoped live behavior inside the proof boundary. |
| `fallback` | `fallback` | The provider selected a conservative fallback path. |
| `block` | `blocked` | The provider refused an unsafe or unsupported action. |
| `defer` | `shadowed` | The provider recorded proof without driving live behavior. |

Future fields such as `preview`, `explanation_only`, or `deferred` may be added
only as additive schema and code changes. Until then, previews and
explanation-only receipts must be expressed through `decision`, `fallback`,
`reason`, and `request_receipt`.

## Fact Source and Confidence Rules

Provider receipts must not make stronger claims than their fact source allows.

| Fact source | Receipt behavior |
|---|---|
| `parser_syntax` | May support source-backed behavior when confidence and freshness guards pass. |
| `legacy_workspace` | May support existing fallback or source-backed behavior inside its legacy claim boundary. |
| `semantic_fact` | May support source-backed behavior when confidence and freshness guards pass. |
| `compiler_fact` | May support source-backed behavior when confidence, freshness, and provider-specific guards pass. |
| `framework_adapter` | Must be labeled or blocked unless a class-specific proof promotes it. |
| `dynamic_boundary` | Must block unsafe edits and exact static claims. |
| `fallback` | Must not be presented as exact compiler or semantic proof. |
| `unknown` | Must remain fallback, blocked, or explanation-only. |

High-confidence fresh source-backed receipts may support scoped live behavior.
Medium-confidence, low-confidence, stale, generated/no-source, dynamic, and
unknown receipts must not authorize edit-producing behavior.

## Blocker Vocabulary

When a provider blocks or refuses behavior, the receipt should expose a known
blocker through `reason`, `blocker`, or a normalized provider-local
`request_receipt` field.

Known blockers are:

```text
generated_no_source
dynamic_boundary
stale_fact
low_confidence
ambiguous_identity
imported_exported
typeglob_alias
autoload
symbolic_ref
dynamic_require
rollback_missing
current_source_reference
workspace_reference
unsupported_fact_class
unsafe_edit_blocked
missing_fact
fallback_policy
unknown
```

Provider-specific blockers may appear in `request_receipt`, but they must not
contradict the normalized `decision`, `reason`, `fallback`, `confidence`, or
`freshness` fields.

## Copyable Payload

`copyable_payload` is a user-initiated support payload. It must include:

- `schema_version = "provider_decision_bug_report.v1"`
- `perl_lsp_version`
- `workspace_root_class`
- `workspace_root_hash` or `null`
- `request_position` or `null`
- `provider`
- `decision`
- `reason`
- `fact_source`
- `confidence`
- `freshness`
- `fallback`
- `dynamic_boundary`
- `support_tier_link`
- `request_receipt` or `null`

It must not include raw workspace roots, raw launch paths, secrets, or ambient
environment values. Workspace identity must be represented by class, count,
hash, or another explicit redaction.

## Valid PR Shapes

Valid PRs under this spec include:

- adding or updating schema snapshots for `provider_decision.v1`
- adding provider-local receipt fields while preserving normalized v1 fields
- adding new provider enum values with schema, tests, and status updates
- adding a validator for the v1 schema
- clarifying user-facing messages without changing canonical fields
- adding copyable payload redaction proof

## Invalid PR Shapes

Invalid PRs include:

- changing provider behavior from a receipt-only PR
- removing required v1 fields
- treating `user_message` as canonical decision state
- presenting generated/no-source, stale, low-confidence, dynamic, or unknown
  facts as exact source-backed proof
- exposing raw workspace paths or environment values in copyable payloads
- adding telemetry or automatic upload of copyable payloads
- promoting support tiers from receipt presentation alone

## Acceptance

A provider decision receipt PR satisfies this spec when:

- receipts conform to the v1 schema
- every receipt has an explicit schema version
- normalized fields do not contradict provider-local receipt fields
- unknown providers produce conservative low-confidence fallback receipts
- caller-supplied `request_receipt` takes precedence over reconstructed state
- copyable payloads redact workspace identity
- support-tier and dashboard wording remain bounded by current receipts

## Proof Commands

Provider decision schema and command proof:

```bash
cargo test -p perl-lsp-rs-core --lib provider_decision --profile agent --locked -- --nocapture
cargo test -p perl-lsp-rs --test lsp_provider_decision_explanation_snap --profile agent --locked -- --nocapture --test-threads=1
cargo test -p perl-lsp-rs --test lsp_execute_command_tests test_execute_command_explain_provider_decision --profile agent --locked -- --nocapture --test-threads=1
```

Docs and support proof for docs-only PRs:

```bash
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
cargo xtask ci-hygiene check-doc-paths docs/specs
git diff --check
```

Schema parsing proof:

```bash
powershell -NoProfile -Command "Get-Content schemas/provider_decision.v1.schema.json -Raw | ConvertFrom-Json | Out-Null"
```

## Non-goals

- No provider behavior change.
- No broad provider cutover.
- No rename or safe-delete authorization change.
- No diagnostic suppression or severity change.
- No workspace scan, Perl probe, `perldoc` run, DAP launch, or subprocess
  behavior change.
- No replacement for provider confidence receipts in
  [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md).
- No replacement for user-facing trust surface boundaries in
  [PLSP-SPEC-0012](PLSP-SPEC-0012-user-facing-trust-surfaces.md).

## Claim Boundaries

Provider decision receipts may claim that `perl-lsp` can explain a specific
provider decision using the available evidence. They may not claim that the
provider is broadly live, broadly compiler-backed, or safe for edit-producing
behavior unless the provider promotion ledger and support tiers already support
that claim.
