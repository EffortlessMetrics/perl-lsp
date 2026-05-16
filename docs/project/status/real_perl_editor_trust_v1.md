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
| Workspace symbols | `partial live source-backed` | Shadow compare records quality candidates; Mojolicious scenario 33 records live-provider query latency, useful/noisy hits, generated candidate gating, dynamic-boundary-shaped names, and edit freshness; Dancer2 scenario 39 adds second-project workspace-symbol noise, generated/dynamic candidate boundary, and edit-freshness proof; Catalyst scenario 41 adds third-project generated/framework candidate, dynamic-boundary-shaped, noise, and edit-freshness proof; runtime requests now record ready-index source-backed compiler-symbol traces for non-empty queries plus a trace-only generated/dynamic/stale/fallback-noise expansion receipt | Ready workspace-index symbols can answer live with high-confidence/source-backed traces; empty-query, partial-index, open-document fallback, stale, dynamic, generated/no-source, and ambiguous compiler candidates stay gated | Narrow generated-symbol pilot proof before broader workspace-symbol expansion |
| Semantic tokens | `partial live token-class pilot` | Mojolicious scenario 34 records live token counts, LSP 5-tuple/span validity, source-backed token hits, dynamic-boundary string non-promotion, and edit freshness; Dancer2 scenario 38 adds second-project package, DSL, app, typeglob-boundary, and edit-freshness token proof; runtime quality receipts now record a source-backed compiler-fact subroutine-declaration class whose span matches the existing live parser/HIR `function` token output; semantic-shadow receipts now cover generated/no-source and fallback token boundaries | Existing parser/token provider remains live; generated/no-source, stale, dynamic, and fallback compiler classifications stay blocked, fallback-only, or shadowed; the compiler-backed pilot emits no new token output and does not authorize broader compiler-backed token classes | Additional project-shaped compiler-backed token-class receipts before broader semantic-token cutover |
| Rename | `partial live lexical + package-local pilot / boundary-shadowed broader compiler facts` | Mojolicious scenario 35 records exact local lexical edits, generated-accessor no-edit boundary, dynamic typeglob-string no-edit boundary, and open-document freshness; Dancer2 scenario 37 adds a second real-workspace unsafe-edit receipt covering exact lexical edits, generated `has` accessor no-edit behavior, dynamic typeglob no-edit behavior, and freshness; #8915 proves a narrow same-file scoped lexical live slice; `lsp_rename_tests::test_workspace_rename_workspace_edit_rolls_back_cleanly` proves scoped qualified multi-file WorkspaceEdits can be inverted exactly; the RealBaseline `helper -> renamed_helper` runtime receipt records live-provider ambiguity plus an imported-symbol compiler blocker and `compiler_blocked` fallback/noise without promotion, and the request-local explain-provider-decision receipt preserves that fallback/noise object for bug-report context; the imported `alias -> renamed_alias` call receipt records live-provider edit noise and `compiler_missing` fallback/noise without promotion; the core package/compiler-backed pilot proof classifies source-backed definition/reference plans, the runtime package-pilot receipt closes the real-workspace empty-plan boundary with a source-backed definition edit, `perl.previewPackageRename` exposes scoped no-edit planned-edit/blocker/fallback UX with explicit rollback/no-edit receipts for imported-symbol blockers, imported-call edit-noise, and compiler-allowed source-backed definition/reference pilot previews, the package-local live-pilot guardrail receipt proves generated, dynamic, stale, and low-confidence blockers still return no edits while preserving source-backed definition/reference planned-edit evidence, and the live package-local pilot now applies only materialized source-backed semantic edit sets that exactly match the workspace source/ambiguity guard | Same-file scoped live rename requires exactly one source-backed `my` or `state` declaration edit; package-local live rename requires fresh source-backed semantic edits that exactly match source/ambiguity guard coverage; stale, low-confidence, generated, dynamic, package-wide, missing compiler proof, ambiguous, and broader compiler-backed facts cannot authorize edits | Post-cutover package-local proof review before broader package/compiler-backed rename promotion |
| Safe delete | `partial live source-backed pilot / boundary-shadowed broader facts` | Mojolicious scenario 36 records file-delete warning UX for a dependent module delete; Dancer2 runtime receipts record symbol-level `_compile`, `routes`, and `plugin_keywords` request shapes where stale, generated, dynamic-boundary, and low-confidence fixtures block deletion with zero live edits; CPAN-style RealBaseline runtime receipts record `RealBaseline::Util::helper` blocked by fresh compiler facts because it is imported by another file and `RealBaseline::Base::reset` allowed by fresh high-confidence semantic facts; requested RealBaseline `reset` edit rollback proof records a source-backed delete WorkspaceEdit plus inverse rollback edit that restores the original text; Dancer2 `to_psgi` now adds a second project-shaped source-backed live-pilot receipt with delete edit and rollback proof; `perl.safeDeleteSymbol` returns delete WorkspaceEdits only when the compiler plan is allowed, the exact source-backed subroutine guard passes, and rollback proof is safe; covered safe-delete receipt paths persist provider decisions that `perl.explainProviderDecision` can replay; `perl.previewSafeDelete` still exposes blocked/allowed scoped no-edit UX | Stale, low-confidence, generated, imported/exported, fallback, non-source-backed, and dynamic facts cannot authorize symbol deletion; the live pilot is limited to source-backed subroutine delete edits with rollback proof; broader safe-delete remains blocked or unsupported | Post-second-project safe-delete proof review before broader symbol-delete promotion |

## Refactor Support Review

Current receipt set does not justify a broad refactor tier promotion. Rename
remains `partial-live-with-fallback`: same-file lexical rename is live only for
the scoped `my`/`state` case, and package-local live rename is limited to exact
source-backed semantic edit sets that match the workspace source/ambiguity
guard. Safe delete is now `partial-live-with-fallback` only for the narrow
source-backed symbol-delete pilot. The recent rollback, live-blocker, and
fallback/noise receipts sharpen known limitations and next proof, but they do
not authorize broad package/compiler-backed rename or broader symbol-level
safe-delete cutover.

## Near-Term PR Order

This dashboard keeps the next provider lane separate from parser capability,
framework facts, PIR, formatter, critic, release, and security work.

1. `docs(status): review second-project safe-delete live-pilot receipt`

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
as user-readable no-edit UX. `perl.safeDeleteSymbol` now exposes a narrow
source-backed live pilot that returns a delete WorkspaceEdit only when compiler
allow proof, source guard, and rollback proof all pass.
`perl.previewPackageRename` now exposes scoped package/compiler-backed rename
previews as user-readable no-edit UX with planned edit evidence plus fallback
or blocker state.
VS Code command palette wiring now exposes provider explanation, safe-delete
preview, and copyable receipt commands without changing provider behavior or
safe-delete edit authorization.

Package-local rename live support has now moved from preview-only to a narrow
pilot. The compiler-allowed preview receipt proves the eligible no-edit UX shape
for source-backed definition/reference plans, real-workspace package-pilot
requests close the empty compiler plan boundary with a source-backed definition
edit, and the package-local live-pilot receipts prove exact source-backed edit
application plus fallback/no-edit guardrails. The live path also requires the
materialized semantic edit set to match the workspace source/ambiguity guard
before returning compiler-backed edits: ambiguous cross-package references are
hard-refused, partial semantic plans fall back to the existing safe
workspace-index path when that guard accepts the request, and generated,
dynamic, stale, low-confidence, package-wide, or missing-proof blockers still
return no edits.
This is a narrow
`partial-live-with-fallback` pilot, not a broad compiler-backed rename
authorization.

Safe-delete support tiers have been reviewed after the scoped preview, edit
rollback proof, and narrow source-backed live pilot: the row is now
`partial-live-with-fallback` only for the exact source-backed symbol-delete
pilot. That cutover does not justify broader symbol deletion. The next proof is
a post-receipt status review of the RealBaseline and Dancer2 source-backed
live-pilot shapes before any broader safe-delete promotion.

Parser raw-bucket work, Linux corpus refresh, security alert classification,
Rust 1.95 rollout, native formatter, native critic, PIR, and determinism remain
separate lanes with their own proof commands and claim boundaries.

Workspace-symbol support has been reviewed after the source-backed ready-index
pilot: the row is now `partial live source-backed` only for non-empty queries
answered from the fresh ready workspace index. Empty-query, partial-index,
open-document fallback, generated/no-source, stale, dynamic, and ambiguous
compiler candidates remain fallback or gated. Catalyst scenario 41 adds the
third real-project generated/dynamic/noise receipt without promoting those
candidate classes.

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
