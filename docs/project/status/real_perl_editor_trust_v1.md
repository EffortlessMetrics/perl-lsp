# Real Perl Editor Trust v1 Dashboard

> Human-owned. This dashboard routes the current Real Perl Editor Trust lane.
> It does not generate metrics, broaden live provider behavior, or replace the
> provider-specific proof surfaces.

Last reviewed: 2026-05-13.

This page answers:

> Which editor surfaces have enough compiler-fact proof to be trusted live,
> which surfaces are still shadowed, and what proof is required next?

Use this page as the routing surface. Use the linked status docs as the source
of current evidence.

## Source Stack

| Need | Source |
| --- | --- |
| User-facing support claims and known limitations | [SUPPORT_TIERS.md](SUPPORT_TIERS.md) |
| Provider fact source, confidence, freshness, fallback, and next proof | [provider_confidence_matrix.md](provider_confidence_matrix.md) |
| Provider live/shadow state and cutover rules | [provider_cutover.md](provider_cutover.md) |
| Compiler-backed provider receipts | [semantic_shadow_compare.md](semantic_shadow_compare.md), [semantic_scorecard.md](semantic_scorecard.md) |
| UX/provider capability context | [ux_capability_dashboard.md](ux_capability_dashboard.md) |
| Real-workspace baseline anchor | [2026-05-13 Mojolicious baseline](../../forensics/2026-05-13-real-workspace-baseline-mojolicious.md) |
| Lane plan and active work | [Real Perl Editor Trust plan](../../../plans/real-perl-editor-trust/implementation-plan.md), [active goal manifest](../../../.perl-lsp/goals/active.toml) |

## Provider Trust Loop

The v1 editor loop is:

```text
completion suggests it
hover explains it
definition jumps to it
references finds its uses
diagnostics trusts it
rename / safe-delete know whether it is safe
symbols and tokens expose project shape without noise
```

The loop is only trusted where each answer can identify its fact source,
confidence, freshness, source-backed range, fallback state, and dynamic-boundary
blocker when relevant.

## Current Dashboard

| Surface | Current state | Real-workspace receipt state | Fallback / blocker coverage | Next proof |
| --- | --- | --- | --- | --- |
| Completion | `partial live / shadowed` | Mojolicious visible-symbol ranking receipt covers candidate counts, top-N churn, useful/noisy additions, generated labels, and dynamic/fallback labels for scenario 28 | Legacy fallback; generated and dynamic-boundary candidates remain shadowed or blocked | Additional project-shape completion quality receipts before broader generated, dynamic, method, or workspace-wide completion cutover |
| Hover | `partial live / provenance-backed` | Mojolicious scenario 29 records exact, imported, generated/framework, dynamic-shaped, module-resolution, and fallback/missing-fact hover surfaces | Legacy fallback; imported, generated, dynamic-boundary, and fallback paths are labeled in receipts | Additional project-shape hover quality receipts before broader generated/dynamic expansion |
| Goto definition | `partial live exact/imported` | Mojolicious scenario 30 records module-resolution, exact-local, imported-symbol, and dynamic-boundary-shaped definition probes | Legacy fallback for generated/no-source, dynamic, stale, low-confidence, and ambiguous candidates | Additional generated/dynamic project-shape receipts with no false-exact source-location claims |
| References | `partial live exact/imported` | Mojolicious scenario 30 records exact-local, imported-symbol, and declaration-including boundary reference probes | Legacy fallback for generated/no-source, declaration-including, coderef, typeglob, dynamic, stale, low-confidence, and ambiguous cases | Precision/recall receipts for generated, coderef, typeglob, dynamic, and broader declaration-including cases |
| Diagnostics | `partial live` | Mojolicious baseline explicitly defers diagnostic correctness | Conservative diagnostics remain when semantic evidence is absent, ambiguous, stale, or dynamic | Broader false-positive / false-negative receipts plus real-workspace diagnostic correctness proof |
| Document symbols | `shadowed` | No dedicated real-workspace document-symbol quality receipt yet | Existing provider remains live; compiler/source/freshness candidates stay shadowed | Real-workspace document-symbol quality receipt, then source-backed compiler document-symbol live slice |
| Workspace symbols | `shadowed` | Shadow compare records quality candidates; noise/rank receipt pending | Existing workspace index remains live; stale/dynamic/generated candidates stay gated | Real-workspace workspace-symbol noise receipt before live high-confidence compiler symbols |
| Semantic tokens | `shadowed` | No dedicated real-workspace semantic-token quality receipt yet | Existing parser/token provider remains live; stale/dynamic compiler classifications stay shadowed | Token/span invariant receipt, then narrow compiler-backed token classes |
| Rename | `boundary-shadowed` | Real-workspace unsafe-edit proof pending | Stale, low-confidence, generated, and dynamic facts cannot authorize edits | Lexical rename live proof plus real-workspace unsafe-edit receipts |
| Safe delete | `boundary-shadowed` | Real-workspace unsafe-delete proof pending | Stale, low-confidence, generated, imported/exported, and dynamic facts cannot authorize deletion | Exact-static safe-delete live proof plus explicit blocker UX receipts |

## Near-Term PR Order

This dashboard keeps the next provider lane separate from parser capability,
framework facts, PIR, formatter, critic, release, and security work.

1. `feat(document-symbols): enable source-backed compiler document symbols`
2. `test(workspace-symbols): add real-workspace compiler symbol noise receipt`
3. `feat(rename): enable lexical rename from ScopeGraph`
4. `feat(framework): add Class::Tiny generated-member facts`

Parser raw-bucket work, Linux corpus refresh, security alert classification,
Rust 1.95 rollout, native formatter, native critic, PIR, and determinism remain
separate lanes with their own proof commands and claim boundaries.

## Promotion Rules

- Do not promote a provider because a fact exists.
- Do not use real-workspace latency alone as correctness proof.
- Do not use shadow receipts as live cutover claims.
- Do not use stale corpus receipts for parser bucket-count movement or support
  promotion.
- Generated and dynamic facts must be labeled or blocked, not silently treated
  as exact static facts.
- Edit-producing providers require real-workspace unsafe-edit/delete receipts
  before broader live behavior.
