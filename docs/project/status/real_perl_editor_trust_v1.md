# Real Perl Editor Trust v1 Dashboard

> Human-owned. This dashboard routes the current Real Perl Editor Trust lane.
> It does not generate metrics, broaden live provider behavior, or replace the
> provider-specific proof surfaces.

Last reviewed: 2026-05-24.

This page answers:

> Which editor surfaces have enough compiler-fact proof to be trusted live,
> which surfaces are still shadowed, and what proof is required next?

Use this page as the routing surface. Use the linked status docs as the source
of current evidence.

## Source Stack

| Need | Source |
| --- | --- |
| Normative v1 boundary and promotion discipline | [PLSP-SPEC-0015](../../specs/PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md) |
| Provider decision receipt schema and explanation payload contract | [PLSP-SPEC-0016](../../specs/PLSP-SPEC-0016-provider-decision-receipt-v1.md), [provider_decision.v1.schema.json](../../../schemas/provider_decision.v1.schema.json) |
| Shared fact provenance and source-backing semantics | [PLSP-SPEC-0017](../../specs/PLSP-SPEC-0017-fact-provenance-and-source-backing.md) |
| Shared edit authorization states for rename and safe delete | [PLSP-SPEC-0018](../../specs/PLSP-SPEC-0018-edit-authorization-contract.md) |
| Semantic token class promotion rules | [PLSP-SPEC-0019](../../specs/PLSP-SPEC-0019-semantic-token-class-promotion-contract.md), [semantic-token-classes.toml](../../../policy/semantic-token-classes.toml) |
| Workspace symbol generated-label rules | [PLSP-SPEC-0020](../../specs/PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md), [workspace-symbol-classes.toml](../../../policy/workspace-symbol-classes.toml) |
| Module path authority and ambient-input boundaries | [PLSP-SPEC-0022](../../specs/PLSP-SPEC-0022-module-path-authority.md), [PLSP-SPEC-0023](../../specs/PLSP-SPEC-0023-ambient-inputs.md) |
| Determinism receipt v1 planning boundary | [PLSP-SPEC-0026](../../specs/PLSP-SPEC-0026-determinism-receipt-v1.md) |
| Differential real-Perl oracle planning boundary | [PLSP-SPEC-0027](../../specs/PLSP-SPEC-0027-differential-real-perl-oracle.md), [oracle_fixture_manifest.v1.schema.json](../../../schemas/oracle_fixture_manifest.v1.schema.json), [oracle fixture manifest](../../../crates/perl-corpus/fixtures/differential_oracle/manifest.json), [oracle_receipt.v1.schema.json](../../../schemas/oracle_receipt.v1.schema.json) |
| User-facing support claims and known limitations | [SUPPORT_TIERS.md](SUPPORT_TIERS.md) |
| Provider fact source, confidence, freshness, fallback, and next proof | [provider_confidence_matrix.md](provider_confidence_matrix.md) |
| Provider promotion, fallback, blocker, and defer decisions by fact class | [provider_promotion_ledger.md](provider_promotion_ledger.md), [provider-promotion-ledger.toml](../../../policy/provider-promotion-ledger.toml) |
| Provider live/shadow state and cutover rules | [provider_cutover.md](provider_cutover.md) |
| Compiler-backed provider receipts | [semantic_shadow_compare.md](semantic_shadow_compare.md), [semantic_scorecard.md](semantic_scorecard.md) |
| UX/provider capability context | [ux_capability_dashboard.md](ux_capability_dashboard.md) |
| Real-workspace baseline anchors | [2026-05-13 Mojolicious baseline](../../forensics/2026-05-13-real-workspace-baseline-mojolicious.md), [2026-05-14 Dancer2 baseline](../../forensics/2026-05-14-real-workspace-baseline-dancer2.md), [2026-05-19 Catalyst baseline](../../forensics/2026-05-19-real-workspace-baseline-catalyst.md) |
| Lane plan and active work | [Real Perl Editor Trust plan](../../../plans/real-perl-editor-trust/implementation-plan.md), [Editor Trust UX closeout plan](../../../plans/editor-trust-ux-closeout/implementation-plan.md), [active goal manifest](../../../.perl-lsp/goals/active.toml) |

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

## Release-Candidate Boundary

Real Perl Editor Trust v1 is now a release-candidate boundary for the current
editor trust surface. It is not a broad provider cutover or a stable/GA product
claim. The boundary freezes the current support posture so future work can use
the promotion ledger instead of widening surfaces by default.

### Live With Fallback

These surfaces may use compiler facts only inside their scoped promotion rules,
and each keeps fallback behavior for unsupported or uncertain cases:

- Completion: partial live visible-symbol support for high-confidence imported
  and exported facts.
- Hover: partial live provenance-backed compiler, framework-adapter, and
  dynamic-boundary explanations.
- Definition, type definition, and references: partial live exact/imported
  source-backed slices with type-definition explanation receipts for the
  existing direct package/class safe subset.
- Diagnostics: partial live suppressions and conservative explanations for
  selected high-confidence semantic evidence.
- Document symbols: partial live source-backed parser-syntax symbols.
- Workspace symbols: partial live source-backed ready-index symbols plus the
  bounded generated-label pilot.
- Semantic tokens: existing parser/HIR output plus narrow source-backed
  compiler-token trace slices that emit no new token output.
- Rename: same-file lexical rename plus the narrow package-local pilot.
- Safe delete: exact unreferenced source-backed subroutine pilot with current
  source, workspace reference, workspace identity, and rollback guards.

### Preview Or Explanation Only

These surfaces are user-facing trust UX, not broader edit authorization:

- `perl.previewPackageRename` exposes planned edits, blockers, fallback state,
  and rollback/no-edit proof without authorizing broad package rename.
- `perl.previewSafeDelete` exposes allowed/blocked no-edit plans when live delete
  proof is incomplete.
- `perl.explainProviderDecision`, diagnostic explanations, missing-module lookup,
  and workspace trust report expose copyable receipts and setup boundaries.
- Workspace trust report setup hints are advisory and derived from existing
  state only; they do not probe Perl, run perldoc, start DAP, or change
  subprocess behavior.

### Blocked Or Deferred

These cases remain blocked, fallback-only, receipt-only, or deferred until a
new row in the promotion ledger names the fact class, promotion rule, fallback
rule, blocker rule, and receipt:

- Broad generated/no-source workspace-symbol promotion.
- Broad compiler-backed semantic-token output.
- Broad package/compiler-backed rename.
- Generated, imported/exported, no-source, dynamic, stale, ambiguous, or
  referenced safe-delete requests.
- Diagnostic correctness claims beyond the scoped explanation and selected
  suppression receipts.
- DAP `includePaths` behavior cutover beyond the current report/config metadata
  boundary.

New compiler facts are substrate only until a provider row promotes one fact
class through the ledger. Receiver-expression facts now have one narrow
source-backed completion pilot; every other receiver class remains substrate,
fallback, or blocked until its own receipt promotes it.

Bareword-classifier facts are compiler substrate only. HIR now records
source-backed syntactic roles for parsed identifier barewords such as static
`require` targets, class receivers, indirect-object targets, qualified names,
and unresolved expression-position barewords. That proof does not change PL109
suppression, diagnostic behavior, provider behavior, parser bucket state,
support tiers, PIR, or determinism claims.

Determinism receipt v1 is now defined as a planning contract. It requires future
receipts to name source snapshots, module-path authority, ambient inputs,
generated/dynamic/stale boundaries, cache/index state, fallback, blockers, and
unknowns before claiming repeatability. The spec does not add a receipt
generator, runtime probe, PIR implementation, provider behavior, support-tier
promotion, release-lineage sync, or determinism claim.

The differential real-Perl oracle contract is now defined as a planning
boundary, its first fixture manifest is declaration-only, and the receipt schema
now locks the future oracle receipt shape. The manifest names fixture
identities, source snapshots, path classes, module roots, environment denials,
dynamic/unsupported boundaries, framework adapters, and comparison classes.
The schema records comparison class, source snapshot, Rust extractor, Perl
oracle, module-path authority, ambient/generated/dynamic/stale/unsupported
inputs, normalized facts, comparison result classes, promotion effect, redaction,
provider-behavior-change flag, and editor-runtime dependency denial. Neither the
manifest nor schema adds an oracle runner, executes Perl, probes workspaces,
changes provider behavior, moves parser/corpus buckets, promotes support tiers,
syncs release lineage, or claims conformance.

## Current Dashboard

| Surface | Current state | Real-workspace receipt state | Fallback / blocker coverage | Next proof |
| --- | --- | --- | --- | --- |
| Completion | `partial live / source-backed receiver pilot / shadowed` | Mojolicious visible-symbol ranking receipt covers candidate counts, top-N churn, useful/noisy additions, generated labels, and dynamic/fallback labels for scenario 28; receiver pilot receipts prove exact source-backed hash-slot receiver ranking, dynamic hash-key fallback preservation, and medium-confidence accessor/method-return fallback preservation; RealReceiver scenario 46 records constructor-assignment and plain hash-slot completion acting source-backed, static package completion acting as exact high-confidence syntax evidence, and hashref-slot, dynamic-key, and unknown receiver probes remaining low-confidence fallback; RealReceiver scenario 47 records accessor-return, method-return, local accessor-chain method-return, dynamic local accessor-chain method-return, and conditional local-reassignment method-return receiver chains preserving low-confidence fallback and tier-6 sorting instead of exact source-backed receiver detail; RealReceiver scenario 48 records literal `bless` receiver evidence as medium-confidence labeled and dynamic `bless` as legacy workspace fallback without exact receiver evidence; RealReceiver scenario 49 records static and dynamic array-index receiver chains preserving low-confidence fallback and tier-6 sorting instead of exact source-backed receiver detail; RealReceiver scenario 50 records `$self`/`$this` local methods as ordinary local method candidates and inherited workspace methods as exact high-confidence self/this receiver evidence without broader receiver promotion | Legacy fallback; unknown, generated/no-source, stale, low-confidence, medium-confidence accessor-return, medium-confidence method-return, medium-confidence local accessor-chain method-return, dynamic local accessor-chain method-return, conditional local-reassignment method-return, medium-confidence literal-bless, unpromoted hashref-slot, unpromoted array-index real-workspace shapes, and dynamic-boundary receiver candidates remain fallback, shadowed, labeled, or blocked; ordinary completion requests persist provider-local decision traces for explain-provider-decision | Additional real-workspace receiver-quality receipts before broader generated, dynamic, method, or workspace-wide completion cutover |
| Hover | `partial live / provenance-backed` | Mojolicious scenario 29 records exact, imported, generated/framework, dynamic-shaped, module-resolution, and fallback/missing-fact hover surfaces | Legacy fallback; imported, generated, dynamic-boundary, and fallback paths are labeled in receipts | Additional project-shape hover quality receipts before broader generated/dynamic expansion |
| Goto definition | `partial live exact/imported` | Mojolicious scenario 30 records module-resolution, exact-local, imported-symbol, and dynamic-boundary-shaped definition probes | Legacy fallback for generated/no-source, dynamic, stale, low-confidence, and ambiguous candidates; ordinary goto-definition requests persist provider-local decision traces for explain-provider-decision | Additional generated/dynamic project-shape receipts with no false-exact source-location claims |
| Type definition | `safe subset + explanation receipt` | Live request receipts now record acted provider-decision traces for existing source-backed direct package/class and constructor receiver results, no-result fallback traces for unproven variable/data-flow receivers, a project-shaped blocker receipt where open-package variable receivers, function-call results, and chained method results stay fallback, an ambiguous package-identity blocker receipt for duplicate open-document package definitions, a generated/dynamic/low-confidence blocker receipt for generated/no-source framework-method, dynamic-method-name, and unknown-receiver request shapes, dynamic boundary blocker receipts for `isa => $type` and `bless {}, $type` shapes, and a stale-fact blocker receipt for stale request versions with request/current document versions | Type-definition behavior is narrowed for ambiguous package identities; variable receivers, chained method results, function-call results, missing packages, duplicate package definitions, generated/no-source facts, dynamic boundaries, stale facts, and low-confidence facts remain fallback, blocked, or unpromoted; dynamic method names, dynamic type constraints, and dynamic bless package arguments are labeled as `dynamic_boundary` in provider-decision receipts, and stale request versions return the existing content-modified error after recording a blocked `stale_fact` receipt with `fallback_state=refresh_workspace_facts` | Additional broader project-shaped blocker receipts before broader type-definition expansion |
| References | `partial live exact/imported` | Mojolicious scenario 30 records exact-local, imported-symbol, and declaration-including boundary reference probes | Legacy fallback for generated/no-source, declaration-including, coderef, typeglob, dynamic, stale, low-confidence, and ambiguous cases; ordinary references requests persist provider-local decision traces for explain-provider-decision | Precision/recall receipts for generated, coderef, typeglob, dynamic, and broader declaration-including cases |
| Diagnostics | `partial live` | Mojolicious baseline explicitly defers broad diagnostic correctness; scenario 31 covers workspace-present imports, a mixed present/missing import boundary, dynamic route-method conservatism, and true missing-module PL701; Dancer2 scenario 40 adds second-project workspace-present import, mixed present/missing import, typeglob-boundary, and true missing-module PL701 proof while scope diagnostics label low-confidence, ambiguous, and dynamic-boundary-shaped visible-symbol evidence when conservative PL109 diagnostics remain; live diagnostic requests now attach an additive `diagnostic_explanation.v1` provider receipt payload with PL701 module-resolution summaries, reported `@INC` path context, trust-boundary labels, and a copyable/user-readable explanation boundary; PL701/PL109 diagnostics expose explain-diagnostic code actions, and `perl.explainMissingModuleLookup` exposes the current missing-module `@INC` lookup as a user-facing receipt | Conservative diagnostics remain when semantic evidence is absent, ambiguous, stale, or dynamic; weak evidence is labeled instead of silently suppressing true unknowns; diagnostic explanation payloads, code actions, and missing-module lookup receipts do not change suppression, severity, resolver behavior, workspace scanning, or support-tier claims | Generated/dynamic diagnostic-label receipts plus broader project-shape false-positive/false-negative proof before wider diagnostic correctness claims; setup-hint/reporting polish before broader diagnostic UX promotion |
| Document symbols | `partial live source-backed` | Runtime quality receipts record source-backed parser-syntax symbol counts and fact traces; Mojolicious scenario 32 records source-backed explicit symbols, generated `has` candidate counts, dynamic-boundary-shaped names, and edit freshness | Astless, stale, dynamic, virtual generated/no-source, low-confidence, and ambiguous candidates keep fallback/gated behavior | Generated-label proof and additional project-shape document-symbol receipts before generated, dynamic, or broader symbol cutover |
| Workspace symbols | `partial live source-backed + generated-label pilot` | Shadow compare records quality candidates; Mojolicious scenario 33 records live-provider query latency, useful/noisy hits, generated candidate gating, dynamic-boundary-shaped names, and edit freshness; Dancer2 scenario 39 adds second-project workspace-symbol noise, generated/dynamic candidate boundary, and edit-freshness proof; Catalyst scenario 41 adds third-project generated/framework candidate, dynamic-boundary-shaped, noise, and edit-freshness proof; Modern OO scenario 43 adds Moose/Moo accessor, delegated-handle, role-composition, method-modifier rank/noise proof with edit freshness and generated/no-source candidate names with zero live exact promotion; runtime requests now record ready-index source-backed compiler-symbol traces plus a labeled source-backed generated/framework pilot receipt for non-empty queries, with separate generated/no-source, dynamic, stale, and fallback/noise gating, the mixed `name` runtime receipt proves source-backed exact symbols rank ahead of generated/framework noise while preserving labels and gated expansion accounting, the false-exact/edit-freshness runtime receipt proves generated pilot symbols stay labeled/source-anchored and refresh after `didChange` while dynamic and stale shadow candidates remain gated, the scoped generated-symbol cutover receipt proves the live response, receipt, source-anchor semantics, and gated expansion boundary agree for the generated/framework member, the Moo predicate generated-member receipt proves another generated-symbol class remains labeled, virtual, source-anchored, and gated against broader generated/dynamic expansion, and the generated/no-source runtime blocker receipt records an unanchored framework/runtime candidate as blocked | Ready workspace-index symbols can answer live with high-confidence/source-backed traces; source-backed generated/framework members may appear only with an explicit generated label anchored to the framework declaration, not as exact generated method bodies; empty-query, partial-index, open-document fallback, stale, dynamic, generated/no-source, and ambiguous compiler candidates stay gated | Additional generated/no-source project variants and explicit-label rank/noise proof before broader generated workspace-symbol expansion |
| Semantic tokens | `partial live source-backed token slice + scoped subroutine/method/package/phase-block/field/method-call/self-method-call/lexical-variable declaration/use traces` | Mojolicious scenario 34 records live token counts, LSP 5-tuple/span validity, source-backed token hits, dynamic-boundary string non-promotion, and edit freshness; Dancer2 scenario 38 adds second-project package, DSL, app, typeglob-boundary, and edit-freshness token proof; Catalyst scenario 42 adds project-shaped false-exact proof for generated/dynamic-looking token shapes plus edit-freshness proof; runtime quality receipts record synthetic, Catalyst-shaped, and RealBaseline source-backed compiler-fact subroutine-declaration classes whose spans match the existing live parser/HIR `function` token output, live requests persist acted provider-decision traces for matched source-backed subroutine-declaration, method-declaration, package-declaration, and phase-block declaration compiler-token slices without adding tokens, the edit-freshness runtime receipt proves `didChange` refreshes live token output and compiler-token identity before recording a fresh post-edit receipt, the live span-invariant proof records decoded token count parity, positive single-line lengths, in-range spans, monotonic ordering, and no overlap, the combined unsafe-boundary shadow receipt proves generated/no-source, dynamic-boundary, stale, and fallback token candidates produce no token identities, the broader compiler-token false-exact receipt proves source-backed `token:method:` compiler spans do not become token identities without class-specific proof, the scoped subroutine-declaration cutover proof allows only source-backed `token:function:` identities whose span already matches exactly one existing live `function` token and proves output-neutral class-specific receipt shape, the scoped method-declaration cutover proof allows only source-backed `token:method_declaration:` identities whose span already matches exactly one existing live `method` token and proves `didChange` freshness without output changes, the scoped package-declaration cutover proof allows only source-backed `token:package_declaration:` identities whose span already matches exactly one existing live `namespace` token and proves `didChange` freshness without output changes, the scoped phase-block declaration cutover proof allows only source-backed `token:phase_block_declaration:` identities whose span already matches exactly one existing live `macro` token and proves `didChange` freshness without output changes, the scoped field-declaration cutover proof allows only source-backed `token:field_declaration:` identities whose span already matches exactly one existing live `variable` token and proves `didChange` freshness without output changes, the scoped method-call cutover proof allows only source-backed `token:method_call:` identities whose span already matches exactly one existing live `method` token and proves `didChange` freshness without output changes, the scoped self-method-call cutover proof allows only source-backed `token:self_method_call:` identities whose span already matches exactly one existing live `method` token and proves `didChange` freshness without output changes, the scoped lexical-variable declaration proof allows only source-backed `token:lexical_variable_declaration:` identities whose span already matches exactly one existing live `variable` token and proves `didChange` freshness without output changes, and the scoped lexical-variable use proof allows only source-backed `token:lexical_variable_use:` identities whose span already matches exactly one existing live `variable` token and proves `didChange` freshness without output changes | Existing parser/token provider remains live; generated/no-source, stale, dynamic-boundary, low-confidence, fallback, broader compiler-token classes, and unmatched compiler classifications stay blocked, fallback-only, receipt-only, or shadowed; the source-backed compiler-token live slices emit no new token output and do not authorize broader compiler-backed token classes | Another scoped compiler-token class proof before broader compiler-token promotion |
| Rename | `partial live lexical + package-local pilot / boundary-shadowed broader compiler facts` | Mojolicious scenario 35 records exact local lexical edits, generated-accessor no-edit boundary, dynamic typeglob-string no-edit boundary, and open-document freshness; Dancer2 scenario 37 adds a second real-workspace unsafe-edit receipt covering exact lexical edits, generated `has` accessor no-edit behavior, dynamic typeglob no-edit behavior, and freshness; #8915 proves a narrow same-file scoped lexical live slice; `lsp_rename_tests::test_workspace_rename_workspace_edit_rolls_back_cleanly` proves scoped qualified multi-file WorkspaceEdits can be inverted exactly; the RealBaseline `helper -> renamed_helper` runtime receipt records live-provider ambiguity plus an imported-symbol compiler blocker and `compiler_blocked` fallback/noise without promotion, and the request-local explain-provider-decision receipt preserves that fallback/noise object for bug-report context; the imported `alias -> renamed_alias` call receipt records live-provider edit noise and `compiler_missing` fallback/noise without promotion; the core package/compiler-backed pilot proof classifies source-backed definition/reference plans, the runtime package-pilot receipt closes the real-workspace empty-plan boundary with a source-backed definition edit, `perl.previewPackageRename` exposes scoped no-edit planned-edit/blocker/fallback UX with explicit rollback/no-edit receipts for imported-symbol blockers, imported-call edit-noise, compiler-allowed source-backed definition/reference pilot previews, and the Dancer2 `to_psgi` source-backed definition preview receipt, the package-local live-pilot guardrail receipt proves generated, dynamic, stale, and low-confidence blockers still return no edits while preserving source-backed definition/reference planned-edit evidence, the RealBaseline imported-symbol false-allow receipt proves the live package-local path returns no edits and records `package_local_live_pilot_blocked` for `helper`, the live package-local pilot applies only materialized source-backed semantic edit sets that exactly match the workspace source/ambiguity guard, the RealBaseline edit-freshness receipt proves a compiler-allowed source-backed definition plan falls back to broader current-source edits, preserves no-edit preview rollback, and refreshes after `didChange`, the Dancer2 edit-freshness receipt proves the source-backed `to_psgi` preview remains rollback-safe and a post-`didChange` same-file reference routes live rename through fresh workspace-index fallback instead of stale compiler-only evidence, and the Catalyst false-allow receipt proves compiler-allowed package-local evidence hard-refuses ambiguous project-shaped identity with zero edits | Same-file scoped live rename requires exactly one source-backed `my` or `state` declaration edit; package-local live rename requires fresh source-backed semantic edits that exactly match source/ambiguity guard coverage; stale, low-confidence, generated, dynamic, package-wide, missing compiler proof, ambiguous, imported/exported, and broader compiler-backed facts cannot authorize edits | Broader package/compiler-backed rename remains deferred; keep project-shaped unsafe-edit and edit-freshness receipts fresh when rename facts change |
| Safe delete | `partial live source-backed pilot / boundary-shadowed broader facts` | Mojolicious scenario 36 records file-delete warning UX for a dependent module delete; Dancer2 runtime receipts record symbol-level `_compile`, `routes`, and `plugin_keywords` request shapes where stale, generated, dynamic-boundary, and low-confidence fixtures block deletion with zero live edits; CPAN-style RealBaseline runtime receipts record `RealBaseline::Util::helper` blocked by fresh compiler facts because it is imported by another file and `RealBaseline::Base::reset` allowed by fresh high-confidence semantic facts; requested RealBaseline `reset` edit rollback proof records a source-backed delete WorkspaceEdit plus inverse rollback edit that restores the original text; Dancer2 `to_psgi` adds a second project-shaped source-backed live-pilot receipt with delete edit and rollback proof; Dancer2 `header` and post-`didChange` `to_psgi` receipts prove project-shaped referenced source-backed methods are refused with zero returned edits; the cross-file `used_target` receipt proves workspace-index references block returned edits; Catalyst `get_action` adds an ambiguous-identity false-allow receipt where compiler-allowed source-backed definition evidence still returns no edits; the non-subroutine/package-wide source-guard receipt proves package variables and package declarations return no edits with `not_source_backed_exact_subroutine_definition`; the generated/dynamic live-command blocker receipt proves `routes` and `plugin_keywords` return no edits with persisted explain-provider receipts; `perl.safeDeleteSymbol` returns delete WorkspaceEdits only when the compiler plan is allowed, the exact source-backed subroutine guard passes, current source and the workspace index have zero references, the workspace identity guard accepts the request, and rollback proof is safe; covered safe-delete receipt paths persist provider decisions that `perl.explainProviderDecision` can replay; `perl.previewSafeDelete` still exposes blocked/allowed scoped no-edit UX | Stale, low-confidence, generated, imported/exported, fallback, ambiguous, non-source-backed, non-subroutine, package-wide, current-source/workspace-index referenced, and dynamic facts cannot authorize symbol deletion; the live pilot is limited to unreferenced source-backed subroutine delete edits with rollback proof and accepted workspace identity; broader safe-delete remains blocked or unsupported | Keep generated/no-source and dynamic blocker receipts fresh; broader symbol-delete remains blocked or deferred |

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
role composition, method modifiers, and generated/no-source candidate names with
zero live exact promotion. The scoped cutover receipt ties the allowed
generated/framework pilot to the live response, source-anchor receipt, and gated
false-exact/dynamic/stale boundary. The Moo predicate generated-symbol receipt
adds one more generated-member class proof while preserving the same virtual,
labeled, source-anchor claim boundary. The generated/no-source runtime receipt
now records blocked runtime-installed and role-composed no-source variants with
zero live exact promotion. The generated-symbol support review is now recorded.
Any broader generated-symbol expansion still needs explicit-label rank/noise
proof. This review does not promote workspace symbols beyond the existing
source-backed ready-index slice plus generated-label pilot.

## Semantic Token Support Review

The scoped token-class receipts do not justify a broader compiler-backed
semantic-token cutover. The subroutine-declaration proof authorizes only the
scoped `token:function:` identity class when its source-backed span already
matches an existing live parser/HIR `function` token. The
method-declaration proof authorizes only the scoped
`token:method_declaration:` identity class when its source-backed span already
matches an existing live parser/HIR `method` token and `didChange` freshness is
proven. The package-declaration proof now authorizes only the scoped
`token:package_declaration:` identity class when its source-backed span already
matches an existing live parser/HIR `namespace` token and `didChange` freshness
is proven. The phase-block declaration proof now authorizes only the scoped
`token:phase_block_declaration:` identity class when its source-backed span
already matches an existing live parser/HIR `macro` token and `didChange`
freshness is proven. The field-declaration proof now authorizes only the scoped
`token:field_declaration:` identity class when its source-backed span already
matches an existing live parser/HIR `variable` token and `didChange` freshness
is proven. The method-call proof now authorizes only the scoped
`token:method_call:` identity class when its source-backed span already matches
an existing live parser/HIR `method` token and `didChange` freshness is proven.
The self-method-call proof now authorizes only the scoped
`token:self_method_call:` identity class when its source-backed span already
matches an existing live parser/HIR `method` token and `didChange` freshness is
proven. The lexical-variable declaration proof now authorizes only the scoped
`token:lexical_variable_declaration:` identity class when its source-backed span
already matches an existing live parser/HIR `variable` token and `didChange`
freshness is proven. The lexical-variable use proof now authorizes only the
scoped `token:lexical_variable_use:` identity class under the same existing
live `variable` token and `didChange` freshness boundary.
Semantic tokens remain
`partial-live-with-fallback` only for existing parser/HIR output plus the narrow
source-backed subroutine-declaration, method-declaration, package-declaration,
phase-block declaration, field-declaration, method-call, self-method-call,
lexical-variable declaration, and lexical-variable use trace slices that emit no
new token output.
The support review is now recorded: subroutine-declaration,
method-declaration, package-declaration, phase-block declaration,
field-declaration, method-call, self-method-call,
lexical-variable declaration, and lexical-variable use proofs stay scoped,
output-neutral, and
fallback-preserving. They do not authorize a broad compiler-backed
semantic-token cutover. The provider promotion ledger now records lexical-variable
declaration and lexical-variable use rows so the class registry, human ledger,
and machine policy share the same fallback and blocker boundary. The next
semantic-token work must either expose another reviewed scoped class through the
user-facing provider-decision trace or add another class with the same
promotion, fallback, blocker, and span-invariant rules.

`class_declaration` has been reviewed as a possible next scoped token class and
is deferred. The source-backed candidate can be described, but the runtime
receipt does not currently prove exact parity with one existing live `class`
token, so its fallback state remains shadowed. That means there is no
`class_declaration` policy row, no provider-promotion ledger row, no support-tier
movement, no live-trace class promotion, and no semantic-token output change.

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

The non-subroutine/package-wide source-guard receipt is now reviewed. It proves
that package variables and package declarations return no edits with a
`not_source_backed_exact_subroutine_definition` decision trace, but it does not
promote safe-delete beyond the exact source-backed subroutine pilot. The
generated/no-source and dynamic live-command blocker receipt is also reviewed:
`routes` and `plugin_keywords` return zero edits through `perl.safeDeleteSymbol`,
persist explain-provider decision receipts, and keep generated/dynamic shapes
blocked instead of authorizing broader symbol deletion.

## Near-Term PR Order

This dashboard keeps the next provider lane separate from parser capability,
framework facts, PIR, formatter, critic, release, and security work.

Recent workspace-symbol, semantic-token, and rename-preview receipts have
refreshed those surfaces without broadening live behavior. The Modern OO
workspace-symbol receipt, generated/no-source blocker receipt, generated-symbol
support review, and scoped generated-symbol cutover receipt close the immediate
workspace-symbol rank/noise, generated/no-source, review, and cutover routing
items. The semantic-token class receipt support review closes the immediate
  semantic-token review routing item. The rename support review closes the
  immediate refactor review routing item without broadening live rename behavior.
  The package-declaration, phase-block, field/method-call, self-method-call, and
  lexical-variable declaration/use live-trace expansions close the current semantic-token
trace-class routing items without emitting new token output.
The class-declaration readiness review keeps `class_declaration` deferred
because exact live-output parity is not proven.
The non-subroutine/package-wide safe-delete source-guard receipt and the
generated/no-source plus dynamic live-command blocker receipt close the current
safe-delete blocker routing items without broadening deletion. The diagnostic
explanation payload, explain-diagnostic code action, and missing-module lookup
command close the current diagnostic explanation routing items without changing
diagnostic suppression, severity, resolver behavior, workspace scanning, or
support tiers.
The workspace trust report setup-hint pass closes the current first-run setup
hint routing item by reporting advisory Perl/PERL5LIB/perldoc/DAP boundaries
from existing state only; it does not resolve Perl, run perldoc, inspect DAP
sessions, or promote broader setup-health claims.
The DAP/perldoc runtime-state pass closes the current trust-report runtime-state
routing item by adding the perldoc oracle contract from configuration without
resolving or running perldoc and by carrying sanitized VS Code client
DAP/perldoc state into the report; it does not start DAP, run perldoc, probe
Perl, inspect debug-session internals, scan workspace files, or promote broader
setup-health claims.
The launch-configuration/module-path parsing pass closes the current
setup-health parsing receipt by carrying sanitized VS Code launch configuration
counts and path classes into the trust report. It does not copy raw launch
paths, start DAP, resolve Perl, probe module paths, inspect debug-session
internals, or promote broader setup-health claims.
The DAP module-path behavior receipt closes the current setup-health behavior
boundary by proving native DAP launch `includePaths` are still report/config
metadata rather than `@INC` authority for syntax-check or debug-launch
subprocesses. Explicit launch `env.PERL5LIB` remains the current module-path
authority. This receipt records the limitation without changing DAP launch
behavior or promoting broader setup-health claims.
The receiver-aware completion pilot closes the current receiver-fact completion
cutover item for one source-backed class: exact hash-slot receiver facts may
rank method candidates above fallback, while dynamic hash keys preserve bounded
fallback and do not become exact receiver evidence. Completion remains
`partial-live-with-fallback`; broader generated, dynamic, unknown,
low-confidence, stale, and workspace-wide method completion still need separate
proof.
The RealReceiver real-workspace receiver-quality receipt now records the next
project-shaped boundary without promotion: constructor-assignment and plain
hash-slot receivers act with source-backed detail, static package receivers act
with exact high-confidence syntax detail, while hashref-slot,
generated/no-source framework-method, dynamic-key, and unknown receiver probes
preserve low-confidence fallback. That receipt measures the gap for broader
receiver promotion; it does not change completion behavior or support-tier
wording.
The follow-up RealReceiver method/accessor fallback receipt records
project-shaped accessor-return, method-return, local accessor-chain method-return,
dynamic local accessor-chain method-return, and conditional local-reassignment
method-return receiver chains as low-confidence fallback with tier-6 sorting.
It keeps medium-confidence, dynamic, and conditional reassignment facts from
silently becoming exact source-backed completion evidence.
The RealReceiver bless confidence receipt records literal `bless` receivers as
medium-confidence labeled evidence and dynamic `bless` receivers as legacy
workspace fallback without exact receiver evidence. It keeps literal/dynamic
`bless` receiver behavior receipt-only and does not promote completion support.
The RealReceiver array-index fallback receipt records static and dynamic
array-index receiver chains as low-confidence fallback with tier-6 sorting. It
keeps array-index receiver behavior receipt-only and does not promote completion
support.
The RealReceiver self/this quality receipt records current-package `$self->`
and `$this->` receiver completion with local-method, inherited-method, and
nearest-shadow boundaries. Local same-file methods remain ordinary local method
candidates, while inherited workspace methods carry exact high-confidence
self/this syntax evidence. It is receipt-only and does not broaden completion
behavior or support-tier wording.
The direct class type-definition safe subset is now recorded in the provider
promotion ledger: direct package/class identifiers and constructor method
receivers may resolve to source-backed open-document package definitions, while
variables, chained method results, function-call results, missing packages,
dynamic boundaries, generated/no-source receivers, stale facts, low-confidence
facts, ambiguous identities, and unsupported receiver shapes stay fallback or
blocked. This records the existing safe subset and does not broaden type
definition into data-flow or return-value inference.
The project-shaped type-definition blocker receipt now proves those
receiver/data-flow boundaries with an open target package: variable receivers,
function-call results, and chained method results still record
`type_definition_not_proven` fallback instead of becoming exact package
locations. The ambiguous package-identity receipt now proves duplicate
open-document package declarations return no exact locations and persist an
`ambiguous_identity` provider-decision blocker instead of acting as a
high-confidence type-definition result. The generated/dynamic/low-confidence
blocker receipt now proves generated/no-source framework-method,
dynamic-method-name, and unknown-receiver request shapes return no exact
type-definition locations, with dynamic method names labeled as a
`dynamic_boundary` provider-decision blocker. The dynamic type-constraint
receipt adds the same no-result boundary for project-shaped `isa => $type`
constraints and labels that fallback as `dynamic_boundary`. The stale-fact
receipt proves stale `textDocument/typeDefinition` request versions return the
existing content-modified error, record zero exact locations, and persist a
blocked `stale_fact` provider-decision receipt with request/current document
versions and `fallback_state=refresh_workspace_facts` for explanation surfaces.
The dynamic
bless receipt adds the same no-result boundary for project-shaped
`bless {}, $type` package arguments and labels that fallback as
`dynamic_boundary`.
The product-level Real Perl Editor Trust smoke receipt now records one
CPAN-style workspace pass across completion, definition, diagnostics, workspace
trust report, safe-delete preview, and explain-provider-decision. It is a
receipt-only smoke: it records acted, fallback, setup-report, and no-edit
refusal surfaces without broadening provider behavior, promoting support tiers,
or moving release lineage.
The constant-provider substrate proof records static `use constant` scalar,
quoted scalar, hash, quoted-hash, and plus-hash extraction plus a completion
shadow receipt that traces constants as fresh `CompilerFact` /
`SemanticAnalyzer` evidence. It does not change live completion behavior,
provider cutover state, support tiers, or broad constant inference claims.
The prototype-table substrate proof records named subroutine prototype content
and precise source ranges as HIR-side facts, and `RegisterPrototype` compile
effects now derive from that table. It does not change provider behavior,
diagnostic suppression, parser bucket status, support tiers, PIR status, or
determinism claims.

The provider promotion ledger parity review is complete: the machine policy and
human ledger currently agree on 17 provider fact-class rows, and the blocker
registry contains 18 normalized blocker entries. That review did not add or
promote fact classes, broaden provider behavior, promote support tiers, move
parser/corpus buckets, sync release lineage, or continue source-repo
development.

1. `test(semantic-tokens): add another scoped compiler-token class receipt only when a new class is ready`
2. `test(dap): add DAP includePaths cutover proof only if native DAP module-path behavior is being promoted`

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
Live diagnostic request traces now carry an additive
`diagnostic_explanation.v1` payload for returned diagnostics, including PL701
module-resolution summaries, reported `@INC` path context, trust-boundary
labels, and a user-facing message; this is explanation-only and does not alter
diagnostic output.
PL701 and PL109 diagnostics now expose an explanation code action, and
`perl.explainMissingModuleLookup` exposes the current missing-module `@INC`
lookup state with a user message, claim boundary, and copyable payload; both are
explanation-only and do not change diagnostic or resolver behavior.
`perl.previewSafeDelete` now exposes scoped safe-delete blocked/allowed previews
as user-readable no-edit UX. `perl.safeDeleteSymbol` now exposes a narrow
source-backed live pilot that returns a delete WorkspaceEdit only when compiler
allow proof, exact source guard, current-source/workspace reference guards,
workspace identity guard, and rollback proof all pass.
`perl.previewPackageRename` now exposes scoped package/compiler-backed rename
previews as user-readable no-edit UX with planned edit evidence plus fallback
or blocker state.
VS Code command palette wiring now exposes provider explanation,
explain-diagnostic, missing-module lookup, safe-delete preview, copyable
receipt, and workspace trust report commands without changing provider
behavior, safe-delete edit authorization, scanning files, probing Perl, or
promoting support tiers.
`perl.workspaceTrustReport` now includes advisory setup hints, explicit Perl
binary/perldoc/DAP probe boundaries, the perldoc oracle contract, sanitized
VS Code client DAP/perldoc runtime state, and launch-configuration/module-path
counts and path classes; the VS Code output renders those hints without copying
raw launch paths, running perldoc, starting DAP, probing Perl, or changing
subprocess behavior.

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
The copyable receipt refresh now proves the RealBaseline and Dancer2
post-`didChange` rename explanations preserve the same fallback/edit-freshness
request receipt inside `copyable_payload.request_receipt` for bug reports.
This is a narrow
`partial-live-with-fallback` pilot, not a broad compiler-backed rename
authorization.

Safe-delete support tiers have now been reviewed after the scoped preview, edit
rollback proof, narrow source-backed live pilot, second project-shaped
source-backed receipt, Dancer2 referenced-source false-allow blockers, and
Catalyst ambiguous-identity false-allow receipt. The row remains
`partial-live-with-fallback` only for the exact unreferenced source-backed
symbol-delete pilot with accepted workspace identity. RealBaseline `reset` and
Dancer2 `to_psgi` prove that the live path can return client-applied delete
WorkspaceEdits with rollback proof for two project shapes; Dancer2 `header` and
current-source `to_psgi` references prove that project-shaped references block
with zero returned edits; Catalyst `get_action` proves compiler-allowed
source-backed definition evidence plus rollback proof still returns no edits
when the workspace identity guard finds ambiguous project-shaped identity; the
package-variable and package-declaration receipt proves the source guard blocks
non-subroutine and package-wide requests with empty edits. The
RealBaseline live UX receipt now records the same referenced-source blocker
through `perl.safeDeleteSymbol`: referenced `helper` returns `blocked` with
`references_exist`, zero returned edits, no server-applied edits, and a
copyable explain-provider-decision payload. These receipts do not justify
broader symbol deletion, generated/dynamic deletion, fallback/no-source
deletion, or server-applied edits.

Parser raw-bucket work, Linux corpus refresh, security alert classification,
Rust 1.95 rollout, native formatter, native critic, PIR implementation,
determinism receipt implementation, and determinism proof remain separate lanes
with their own proof commands and claim boundaries.

Workspace-symbol support has been reviewed after the source-backed ready-index
pilot and labeled generated/framework pilot. The row remains
`partial live source-backed + generated-label pilot` for non-empty fresh
ready-index queries only. Generated/framework symbols are virtual, labeled, and
anchored to framework declarations rather than exact generated method bodies.
Empty-query, partial-index, open-document fallback, generated/no-source, stale,
dynamic, ambiguous, and fallback/noise candidates remain fallback or gated.
The scoped generated-symbol cutover receipt and Moo predicate generated-symbol
class receipt are now recorded, and the generated/no-source variant receipt
records runtime-installed and role-composed blocked candidates. The next
workspace-symbol work is not another support review; any broader generated
workspace-symbol expansion still needs a new explicit-label rank/noise receipt
and a promotion-ledger row.

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
