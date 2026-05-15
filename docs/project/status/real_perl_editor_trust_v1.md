# Real Perl Editor Trust v1 Dashboard

> Human-owned. This dashboard routes the current Real Perl Editor Trust lane.
> It does not generate metrics, broaden live provider behavior, or replace the
> provider-specific proof surfaces.

Last reviewed: 2026-05-15.

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
| Real-workspace baseline anchors | [2026-05-13 Mojolicious baseline](../../forensics/2026-05-13-real-workspace-baseline-mojolicious.md), [2026-05-14 Dancer2 baseline](../../forensics/2026-05-14-real-workspace-baseline-dancer2.md) |
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
explain-provider-decision exposes the receipt boundary
```

The loop is only trusted where each answer can identify its fact source,
confidence, freshness, source-backed range, fallback state, and dynamic-boundary
blocker when relevant.

## Current Dashboard

| Surface | Current state | Real-workspace receipt state | Fallback / blocker coverage | Next proof |
| --- | --- | --- | --- | --- |
| Completion | `partial live / shadowed` | Mojolicious visible-symbol ranking receipt covers candidate counts, top-N churn, useful/noisy additions, generated labels, and dynamic/fallback labels for scenario 28 | Legacy fallback; generated and dynamic-boundary candidates remain shadowed or blocked; ordinary completion requests persist provider-local decision traces for explain-provider-decision | Additional project-shape completion quality receipts before broader generated, dynamic, method, or workspace-wide completion cutover |
| Hover | `partial live / provenance-backed` | Mojolicious scenario 29 records exact, imported, generated/framework, dynamic-shaped, module-resolution, and fallback/missing-fact hover surfaces | Legacy fallback; imported, generated, dynamic-boundary, and fallback paths are labeled in receipts | Additional project-shape hover quality receipts before broader generated/dynamic expansion |
| Goto definition | `partial live exact/imported` | Mojolicious scenario 30 records module-resolution, exact-local, imported-symbol, and dynamic-boundary-shaped definition probes | Legacy fallback for generated/no-source, dynamic, stale, low-confidence, and ambiguous candidates; ordinary goto-definition requests persist provider-local decision traces for explain-provider-decision | Additional generated/dynamic project-shape receipts with no false-exact source-location claims |
| References | `partial live exact/imported` | Mojolicious scenario 30 records exact-local, imported-symbol, and declaration-including boundary reference probes | Legacy fallback for generated/no-source, declaration-including, coderef, typeglob, dynamic, stale, low-confidence, and ambiguous cases; ordinary references requests persist provider-local decision traces for explain-provider-decision | Precision/recall receipts for generated, coderef, typeglob, dynamic, and broader declaration-including cases |
| Diagnostics | `partial live` | Mojolicious baseline explicitly defers broad diagnostic correctness; scenario 31 covers workspace-present imports, a mixed present/missing import boundary, dynamic route-method conservatism, and true missing-module PL701; Dancer2 scenario 40 adds second-project workspace-present import, mixed present/missing import, typeglob-boundary, and true missing-module PL701 proof while scope diagnostics label low-confidence, ambiguous, and dynamic-boundary-shaped visible-symbol evidence when conservative PL109 diagnostics remain | Conservative diagnostics remain when semantic evidence is absent, ambiguous, stale, or dynamic; weak evidence is labeled instead of silently suppressing true unknowns | Generated/dynamic diagnostic-label receipts plus broader project-shape false-positive/false-negative proof before wider diagnostic correctness claims |
| Document symbols | `partial live source-backed` | Runtime quality receipts record source-backed parser-syntax symbol counts and fact traces; Mojolicious scenario 32 records source-backed explicit symbols, generated `has` candidate counts, dynamic-boundary-shaped names, and edit freshness | Astless, stale, dynamic, virtual generated/no-source, low-confidence, and ambiguous candidates keep fallback/gated behavior | Generated-label proof and additional project-shape document-symbol receipts before generated, dynamic, or broader symbol cutover |
| Workspace symbols | `partial live source-backed` | Shadow compare records quality candidates; Mojolicious scenario 33 records live-provider query latency, useful/noisy hits, generated candidate gating, dynamic-boundary-shaped names, and edit freshness; Dancer2 scenario 39 adds second-project workspace-symbol noise, generated/dynamic candidate boundary, and edit-freshness proof; runtime requests now record ready-index source-backed compiler-symbol traces for non-empty queries | Ready workspace-index symbols can answer live with high-confidence/source-backed traces; empty-query, partial-index, open-document fallback, stale, dynamic, generated/no-source, and ambiguous compiler candidates stay gated | Additional generated/dynamic/noise receipts before broader workspace-symbol expansion |
| Semantic tokens | `shadowed` | Mojolicious scenario 34 records live token counts, LSP 5-tuple/span validity, source-backed token hits, dynamic-boundary string non-promotion, and edit freshness; Dancer2 scenario 38 adds second-project package, DSL, app, typeglob-boundary, and edit-freshness token proof | Existing parser/token provider remains live; stale/dynamic compiler classifications stay shadowed | Narrow compiler-backed token classes before live cutover |
| Rename | `partial live lexical / boundary-shadowed compiler facts` | Mojolicious scenario 35 records exact local lexical edits, generated-accessor no-edit boundary, dynamic typeglob-string no-edit boundary, and open-document freshness; Dancer2 scenario 37 adds a second real-workspace unsafe-edit receipt covering exact lexical edits, generated `has` accessor no-edit behavior, dynamic typeglob no-edit behavior, and freshness; #8915 proves a narrow same-file scoped lexical live slice; `lsp_rename_tests::test_workspace_rename_workspace_edit_rolls_back_cleanly` proves scoped qualified multi-file WorkspaceEdits can be inverted exactly; the RealBaseline `helper -> renamed_helper` runtime receipt records live-provider ambiguity and `compiler_empty` fallback/noise without promotion, and the request-local explain-provider-decision receipt preserves that fallback/noise object for bug-report context; the imported `alias -> renamed_alias` call receipt records live-provider edit noise and `compiler_missing` fallback/noise without promotion; the core package/compiler-backed pilot proof classifies source-backed definition/reference plans but remains receipt-only | Same-file scoped live rename requires exactly one source-backed `my` or `state` declaration edit; stale, low-confidence, generated, dynamic, package-wide, empty fallback, missing compiler proof, and broader compiler-backed facts cannot authorize edits | Real-workspace package/compiler-backed pilot receipt plus support review before broader rename migration |
| Safe delete | `boundary-shadowed` | Mojolicious scenario 36 records file-delete warning UX for a dependent module delete; Dancer2 runtime receipts record symbol-level `_compile`, `routes`, and `plugin_keywords` request shapes where stale, generated, dynamic-boundary, and low-confidence fixtures block deletion with zero live edits; CPAN-style RealBaseline runtime receipts record `RealBaseline::Util::helper` blocked by fresh compiler facts because it is imported by another file and `RealBaseline::Base::reset` allowed by fresh high-confidence semantic facts, with explicit no-live-edit rollback state for both paths; covered safe-delete receipt paths persist trace-only provider decisions that `perl.explainProviderDecision` can replay; `perl.previewSafeDelete` exposes blocked/allowed scoped no-edit UX for those paths | Stale, low-confidence, generated, imported/exported, and dynamic facts cannot authorize symbol deletion; allowed semantic proof still does not enable live symbol deletion; safe-delete decision traces and previews are explanations, not edit authorization | Actual symbol-delete edit cutover proof with rollback before promotion |

## Refactor Support Review

Current receipt set does not justify a broader refactor tier promotion.
Rename remains `partial-live-with-fallback` for the narrow same-file lexical
slice only. Safe delete remains `shadowed` for symbol-level deletion. The
recent rollback, live-blocker, and fallback/noise receipts sharpen known
limitations and next proof, but they do not authorize package/compiler-backed
rename or live symbol-level safe-delete cutover.

## Near-Term PR Order

This dashboard keeps the next provider lane separate from parser capability,
framework facts, PIR, formatter, critic, release, and security work.

1. `feat(vscode): expose provider explanation commands`
2. `test(rename): add real-workspace package/compiler-backed pilot receipt`
3. `feat(workspace-symbols): enable high-confidence compiler symbols`
4. `test(semantic-tokens): add compiler-backed token class receipt`

Provider decision explanations are already partial-live through
`perl.explainProviderDecision`; callers can attach a request-local
`request_receipt` object for bug reports, existing live rename paths now record
provider-local traces, and covered refactor runtime receipt surfaces persist
provider-local traces that the command can replay when the caller does not
provide a receipt. Ordinary live completion, goto-definition, references,
hover, diagnostic, document-symbol, workspace-symbol, and semantic-token
requests now persist the same trace model. Navigation and dispatcher traces are
trace-only, low-confidence request-shape receipts; they do not replace
surface-specific compiler proof, and dispatcher traces deliberately do not
overwrite completion's richer provider-local receipt. Safe-delete runtime
blocker receipt paths now persist trace-only allowed, blocked, and fallback
decisions with fact source, confidence, freshness, fallback, blocker, and
claim-boundary fields. Provider explanations and attached request receipts now
carry the additive `provider_decision.v1` schema version plus normalized
fallback, source-backed, and dynamic-boundary fields while preserving
provider-specific receipt fields. `perl.explainProviderDecision` also includes a
formatted `user_message` for command-palette/output-channel use and a local
`copyable_payload` with `perl-lsp` version, redacted workspace-root class/hash,
request position when supplied, support-tier link, and normalized receipt
context for bug reports.
`perl.previewSafeDelete` now exposes scoped safe-delete blocked/allowed previews
as user-readable no-edit UX.

Safe-delete support tiers have been reviewed after the scoped preview proof:
the row remains `shadowed`, and the next proof is actual symbol-delete edit
cutover proof with rollback before promotion.

Parser raw-bucket work, Linux corpus refresh, security alert classification,
Rust 1.95 rollout, native formatter, native critic, PIR, and determinism remain
separate lanes with their own proof commands and claim boundaries.

Workspace-symbol support has been reviewed after the source-backed ready-index
pilot: the row is now `partial live source-backed` only for non-empty queries
answered from the fresh ready workspace index. Empty-query, partial-index,
open-document fallback, generated/no-source, stale, dynamic, and ambiguous
compiler candidates remain fallback or gated.

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
