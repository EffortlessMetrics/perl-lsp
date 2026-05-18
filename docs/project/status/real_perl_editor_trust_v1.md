# Real Perl Editor Trust v1 Dashboard

> Human-owned. This dashboard routes the current Real Perl Editor Trust lane.
> It does not generate metrics, broaden live provider behavior, or replace the
> provider-specific proof surfaces.

Last reviewed: 2026-05-18.

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
| Provider promotion, fallback, blocker, and defer decisions by fact class | [provider_promotion_ledger.md](provider_promotion_ledger.md) |
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
| Workspace symbols | `partial live source-backed + generated-label pilot` | Shadow compare records quality candidates; Mojolicious scenario 33 records live-provider query latency, useful/noisy hits, generated candidate gating, dynamic-boundary-shaped names, and edit freshness; Dancer2 scenario 39 adds second-project workspace-symbol noise, generated/dynamic candidate boundary, and edit-freshness proof; Catalyst scenario 41 adds third-project generated/framework candidate, dynamic-boundary-shaped, noise, and edit-freshness proof; Modern OO scenario 43 adds Moose/Moo accessor, delegated-handle, role-composition, method-modifier rank/noise proof with edit freshness; runtime requests now record ready-index source-backed compiler-symbol traces plus a labeled source-backed generated/framework pilot receipt for non-empty queries, with separate generated/no-source, dynamic, stale, and fallback/noise gating, the mixed `name` runtime receipt proves source-backed exact symbols rank ahead of generated/framework noise while preserving labels and gated expansion accounting, the false-exact/edit-freshness runtime receipt proves generated pilot symbols stay labeled/source-anchored and refresh after `didChange` while dynamic and stale shadow candidates remain gated, the scoped generated-symbol cutover receipt proves the live response, receipt, source-anchor semantics, and gated expansion boundary agree for the generated/framework member, and the Moo predicate generated-member receipt proves another generated-symbol class remains labeled, virtual, source-anchored, and gated against broader generated/dynamic expansion | Ready workspace-index symbols can answer live with high-confidence/source-backed traces; source-backed generated/framework members may appear only with an explicit generated label anchored to the framework declaration, not as exact generated method bodies; empty-query, partial-index, open-document fallback, stale, dynamic, generated/no-source, and ambiguous compiler candidates stay gated | Project-shaped generated/no-source proof before broader generated workspace-symbol expansion |
| Semantic tokens | `partial live source-backed token slice + support-reviewed scoped method/package/field/method-call proof` | Mojolicious scenario 34 records live token counts, LSP 5-tuple/span validity, source-backed token hits, dynamic-boundary string non-promotion, and edit freshness; Dancer2 scenario 38 adds second-project package, DSL, app, typeglob-boundary, and edit-freshness token proof; Catalyst scenario 42 adds project-shaped false-exact proof for generated/dynamic-looking token shapes plus edit-freshness proof; runtime quality receipts record synthetic, Catalyst-shaped, and RealBaseline source-backed compiler-fact subroutine-declaration classes whose spans match the existing live parser/HIR `function` token output, live requests persist acted provider-decision traces for matched source-backed subroutine-declaration and method-declaration compiler-token slices without adding tokens, the edit-freshness runtime receipt proves `didChange` refreshes live token output and compiler-token identity before recording a fresh post-edit receipt, the live span-invariant proof records decoded token count parity, positive single-line lengths, in-range spans, monotonic ordering, and no overlap, the combined unsafe-boundary shadow receipt proves generated/no-source, dynamic-boundary, stale, and fallback token candidates produce no token identities, the broader compiler-token false-exact receipt proves source-backed `token:method:` compiler spans do not become token identities without class-specific proof, the scoped method-declaration cutover proof allows only source-backed `token:method_declaration:` identities whose span already matches exactly one existing live `method` token and proves `didChange` freshness without output changes, the scoped package-declaration cutover proof allows only source-backed `token:package_declaration:` identities whose span already matches exactly one existing live `namespace` token and proves `didChange` freshness without output changes, the scoped field-declaration cutover proof allows only source-backed `token:field_declaration:` identities whose span already matches exactly one existing live `variable` token and proves `didChange` freshness without output changes, and the scoped method-call cutover proof allows only source-backed `token:method_call:` identities whose span already matches exactly one existing live `method` token and proves `didChange` freshness without output changes | Existing parser/token provider remains live; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader compiler-token classes, and unmatched compiler classifications stay blocked, fallback-only, receipt-only, or shadowed; the source-backed compiler-token live slices emit no new token output and do not authorize broader compiler-backed token classes | Another user-facing live-trace expansion or scoped class proof before broader compiler-token promotion |
| Rename | `partial live lexical + package-local pilot / boundary-shadowed broader compiler facts` | Mojolicious scenario 35 records exact local lexical edits, generated-accessor no-edit boundary, dynamic typeglob-string no-edit boundary, and open-document freshness; Dancer2 scenario 37 adds a second real-workspace unsafe-edit receipt covering exact lexical edits, generated `has` accessor no-edit behavior, dynamic typeglob no-edit behavior, and freshness; #8915 proves a narrow same-file scoped lexical live slice; `lsp_rename_tests::test_workspace_rename_workspace_edit_rolls_back_cleanly` proves scoped qualified multi-file WorkspaceEdits can be inverted exactly; the RealBaseline `helper -> renamed_helper` runtime receipt records live-provider ambiguity plus an imported-symbol compiler blocker and `compiler_blocked` fallback/noise without promotion, and the request-local explain-provider-decision receipt preserves that fallback/noise object for bug-report context; the imported `alias -> renamed_alias` call receipt records live-provider edit noise and `compiler_missing` fallback/noise without promotion; the core package/compiler-backed pilot proof classifies source-backed definition/reference plans, the runtime package-pilot receipt closes the real-workspace empty-plan boundary with a source-backed definition edit, `perl.previewPackageRename` exposes scoped no-edit planned-edit/blocker/fallback UX with explicit rollback/no-edit receipts for imported-symbol blockers, imported-call edit-noise, compiler-allowed source-backed definition/reference pilot previews, and the Dancer2 `to_psgi` source-backed definition preview receipt, the package-local live-pilot guardrail receipt proves generated, dynamic, stale, and low-confidence blockers still return no edits while preserving source-backed definition/reference planned-edit evidence, the RealBaseline imported-symbol false-allow receipt proves the live package-local path returns no edits and records `package_local_live_pilot_blocked` for `helper`, the live package-local pilot applies only materialized source-backed semantic edit sets that exactly match the workspace source/ambiguity guard, the RealBaseline edit-freshness receipt proves a compiler-allowed source-backed definition plan falls back to broader current-source edits, preserves no-edit preview rollback, and refreshes after `didChange`, the Dancer2 edit-freshness receipt proves the source-backed `to_psgi` preview remains rollback-safe and a post-`didChange` same-file reference routes live rename through fresh workspace-index fallback instead of stale compiler-only evidence, and the Catalyst false-allow receipt proves compiler-allowed package-local evidence hard-refuses ambiguous project-shaped identity with zero edits | Same-file scoped live rename requires exactly one source-backed `my` or `state` declaration edit; package-local live rename requires fresh source-backed semantic edits that exactly match source/ambiguity guard coverage; stale, low-confidence, generated, dynamic, package-wide, missing compiler proof, ambiguous, imported/exported, and broader compiler-backed facts cannot authorize edits | Broader package/compiler-backed rename remains deferred; keep project-shaped unsafe-edit and edit-freshness receipts fresh when rename facts change |
| Safe delete | `partial live source-backed pilot / boundary-shadowed broader facts` | Mojolicious scenario 36 records file-delete warning UX for a dependent module delete; Dancer2 runtime receipts record symbol-level `_compile`, `routes`, and `plugin_keywords` request shapes where stale, generated, dynamic-boundary, and low-confidence fixtures block deletion with zero live edits; CPAN-style RealBaseline runtime receipts record `RealBaseline::Util::helper` blocked by fresh compiler facts because it is imported by another file and `RealBaseline::Base::reset` allowed by fresh high-confidence semantic facts; requested RealBaseline `reset` edit rollback proof records a source-backed delete WorkspaceEdit plus inverse rollback edit that restores the original text; Dancer2 `to_psgi` now adds a second project-shaped source-backed live-pilot receipt with delete edit and rollback proof; `perl.safeDeleteSymbol` returns delete WorkspaceEdits only when the compiler plan is allowed, the exact source-backed subroutine guard passes, and rollback proof is safe; covered safe-delete receipt paths persist provider decisions that `perl.explainProviderDecision` can replay; `perl.previewSafeDelete` still exposes blocked/allowed scoped no-edit UX | Stale, low-confidence, generated, imported/exported, fallback, non-source-backed, and dynamic facts cannot authorize symbol deletion; the live pilot is limited to source-backed subroutine delete edits with rollback proof; broader safe-delete remains blocked or unsupported | Additional project-shaped false-allow and blocker receipts before broader symbol-delete promotion |

## Workspace Symbol Support Review

Generated-symbol support remains a bounded labeled pilot. The Mojolicious,
Dancer2, Catalyst, and Modern OO receipts plus runtime rank, false-exact, and
edit-freshness proof justify the current `partial-live-with-fallback` row:
source-backed generated/framework members may appear only as explicit virtual
symbols anchored to framework declarations. They do not justify exact generated
method-body locations, generated/no-source promotion, dynamic-boundary
promotion, stale-fact promotion, partial-index promotion, or open-document
fallback promotion.

The Modern OO receipt covers the requested additional project-shaped
generated-symbol rank/noise proof for Moose/Moo accessors, delegated handles,
role composition, and method modifiers. The scoped cutover receipt ties the
allowed generated/framework pilot to the live response, source-anchor receipt,
and gated false-exact/dynamic/stale boundary. The Moo predicate generated-symbol
receipt adds one more generated-member class proof while preserving the same
virtual, labeled, source-anchor claim boundary. The generated-symbol support
review is now recorded. Any broader generated-symbol expansion still needs
project-shaped generated/no-source proof. This review does not promote
workspace symbols beyond the existing source-backed ready-index slice plus
generated-label pilot.

## Semantic Token Support Review

The scoped token-class receipts do not justify a broader compiler-backed
semantic-token cutover. The
method-declaration proof authorizes only the scoped
`token:method_declaration:` identity class when its source-backed span already
matches an existing live parser/HIR `method` token and `didChange` freshness is
proven. The package-declaration proof now authorizes only the scoped
`token:package_declaration:` identity class when its source-backed span already
matches an existing live parser/HIR `namespace` token and `didChange` freshness
is proven. The field-declaration proof now authorizes only the scoped
`token:field_declaration:` identity class when its source-backed span already
matches an existing live parser/HIR `variable` token and `didChange` freshness
is proven. The method-call proof now authorizes only the scoped
`token:method_call:` identity class when its source-backed span already matches
an existing live parser/HIR `method` token and `didChange` freshness is proven.
Semantic tokens remain
`partial-live-with-fallback` only for existing parser/HIR output plus the narrow
source-backed subroutine-declaration and method-declaration trace slices that
emit no new token output.
The support review is now recorded: method-declaration, package-declaration,
field-declaration, and method-call proofs stay scoped, output-neutral, and
fallback-preserving. They do not authorize a broad compiler-backed semantic-token
cutover. The next semantic-token work must either expose another reviewed
scoped class through the user-facing provider-decision trace or add another class with
the same promotion, fallback, blocker, and span-invariant rules.

## Refactor Support Review

Post-cutover review does not justify a broad refactor tier promotion. Rename
remains `partial-live-with-fallback`: same-file lexical rename is live only for
the scoped `my`/`state` case, and package-local live rename is limited to exact
source-backed semantic edit sets that match the workspace source/ambiguity
guard. The Dancer2 fallback/edit-freshness receipt has now been reviewed and
does not justify broader package/compiler-backed rename promotion. Safe
delete is now `partial-live-with-fallback` only for
the narrow source-backed symbol-delete pilot. The recent rollback,
live-blocker, and fallback/noise receipts sharpen known limitations and next
proof, but they do not authorize broad package/compiler-backed rename or
broader symbol-level safe-delete cutover.

## Near-Term PR Order

This dashboard keeps the next provider lane separate from parser capability,
framework facts, PIR, formatter, critic, release, and security work.

Recent workspace-symbol, semantic-token, and rename-preview receipts have
refreshed those surfaces without broadening live behavior. The Modern OO
workspace-symbol receipt plus generated-symbol support review and scoped
generated-symbol cutover receipt close the immediate workspace-symbol
rank/noise, review, and cutover routing item, and the semantic-token class
receipt support review closes the immediate semantic-token review routing item.
The rename support review closes the immediate refactor review routing item
without broadening live rename behavior.

1. `feat(semantic-tokens): expose another reviewed scoped token class through live provider-decision traces`
2. `test(safe-delete): add additional project-shaped false-allow and blocker receipts`
3. `test(workspace-symbols): add project-shaped generated/no-source proof before broader generated-symbol expansion`

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
preview, copyable receipt, and workspace trust report commands without changing
provider behavior, safe-delete edit authorization, scanning files, probing Perl,
or promoting support tiers.

Package-local rename live support has now moved from preview-only to a narrow
pilot. The compiler-allowed preview receipt proves the eligible no-edit UX shape
for source-backed definition/reference plans, real-workspace package-pilot
requests close the empty compiler plan boundary with a source-backed definition
edit, and the package-local live-pilot receipts prove exact source-backed edit
application plus fallback/no-edit guardrails. The RealBaseline imported-symbol
false-allow receipt proves the live path refuses `helper` with no edits and a
`package_local_live_pilot_blocked` trace instead of treating an imported/exported
fact as package-local. The live path also requires the
materialized semantic edit set to match the workspace source/ambiguity guard
before returning compiler-backed edits: ambiguous cross-package references are
hard-refused, partial semantic plans fall back to the existing safe
workspace-index path when that guard accepts the request, and generated,
dynamic, stale, low-confidence, package-wide, or missing-proof blockers still
return no edits.
The RealBaseline false-allow receipt now proves that a compiler-allowed
source-backed definition plan does not authorize the narrower package-local
pilot when current workspace/source coverage finds broader references, preserves
no-edit preview rollback, and refreshes fallback edits after `didChange`.
The Dancer2 edit-freshness receipt adds the same current-source freshness proof
for `to_psgi`: the preview remains rollback-safe and no-edit, while the live
path uses fresh workspace-index fallback after `didChange` adds a same-file
reference.
This is a narrow
`partial-live-with-fallback` pilot, not a broad compiler-backed rename
authorization.

Safe-delete support tiers have now been reviewed after the scoped preview, edit
rollback proof, narrow source-backed live pilot, and second project-shaped
source-backed receipt. The row remains `partial-live-with-fallback` only for
the exact source-backed symbol-delete pilot. RealBaseline `reset` and Dancer2
`to_psgi` prove that the live path can return client-applied delete
WorkspaceEdits with rollback proof for two project shapes; they do not justify
broader symbol deletion, generated/dynamic deletion, non-subroutine deletion,
package-wide deletion, fallback/no-source deletion, or server-applied edits.

Parser raw-bucket work, Linux corpus refresh, security alert classification,
Rust 1.95 rollout, native formatter, native critic, PIR, and determinism remain
separate lanes with their own proof commands and claim boundaries.

Workspace-symbol support has been reviewed after the source-backed ready-index
pilot and labeled generated/framework pilot. The row remains
`partial live source-backed + generated-label pilot` for non-empty fresh
ready-index queries only. Generated/framework symbols are virtual, labeled, and
anchored to framework declarations rather than exact generated method bodies.
Empty-query, partial-index, open-document fallback, generated/no-source, stale,
dynamic, ambiguous, and fallback/noise candidates remain fallback or gated.
The scoped generated-symbol cutover receipt and Moo predicate generated-symbol
class receipt are now recorded; the next workspace-symbol decision is support
review before any broader generated workspace-symbol expansion.

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
