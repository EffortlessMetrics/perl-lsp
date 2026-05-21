# Provider Cutover Status

> Human-owned. This page tracks when LSP providers consume compiler facts.
> Fact availability alone is not a provider cutover.

Provider cutover is intentionally staged. New compiler facts should first be
fixture-backed, then shadowed or scorecarded, then consumed by a provider with
fallback behavior and rollback proof.

For a row-per-provider receipt summary with fact source, confidence, freshness,
fallback, runtime comparison, live state, and next proof, see
[provider confidence matrix](provider_confidence_matrix.md).
For the Real Perl Editor Trust v1 routing dashboard that ties provider state to
support claims, real-workspace receipts, and next PRs, see
[real_perl_editor_trust_v1.md](real_perl_editor_trust_v1.md).
For class-level promote, fallback, block, and defer decisions, see
[provider promotion ledger](provider_promotion_ledger.md).

## Recent Proof

- Fact-source trace receipt wiring is in place through `ProviderFactTrace`
  entries in the semantic shadow compare receipt schema.
- The current semantic-shadow compare artifact records fifty deterministic
  receipts across definition, references, completion, hover, diagnostics, workspace-symbol,
  document-symbol, semantic-token, rename, and safe-delete surfaces.
- Definition/reference shadow proof now records imported-symbol,
  framework-generated, dynamic-boundary, low-confidence fallback, stale fact, and
  real-workspace quality candidate/occurrence traces.
- Definition now has a narrow live exact/imported runtime slice: a single
  fresh, high-confidence, source-backed `ExactAst`, explicit import, default
  export, or export-tag candidate can drive `textDocument/definition`;
  generated/no-source, dynamic, stale, low-confidence, and ambiguous candidates
  retain legacy fallback.
- References now have a narrow live source-backed runtime slice: fresh,
  high-confidence, source-backed `ExactAst`, `ImportExportInference`, or
  `LiteralRequireImport` occurrence references can drive
  `textDocument/references` when declaration inclusion is off; generated/no-source,
  dynamic, stale, low-confidence, ambiguous, and declaration-including requests
  retain legacy fallback.
- Completion shadow proof now records compiler visible-symbol candidate deltas,
  generated-member labels, and dynamic-boundary blockers. A narrow live
  completion slice promotes only high-confidence imported/exported
  visible-symbol facts; generated and dynamic-boundary candidates remain
  shadowed or blocked.
- The Mojolicious scenario 28 completion ranking receipt now records
  real-workspace visible-symbol candidate counts, top-N churn, useful/noisy
  additions, generated labels, and dynamic/fallback labels without broadening
  live completion behavior.
- The Mojolicious scenario 29 hover provenance receipt now records exact,
  imported, generated/framework, dynamic-shaped, module-resolution, and
  fallback/missing-fact hover surfaces without broadening hover behavior.
- The Mojolicious scenario 30 navigation quality receipt now records
  definition/reference result counts and valid LSP shapes for module-resolution,
  exact-local, imported-symbol, dynamic-boundary-shaped, and
  declaration-including probes without broadening navigation behavior.
- Hover provenance proof now records imported-symbol, framework-generated,
  dynamic-boundary, and fallback paths with typed fact-source traces and source /
  confidence labels.
- Hover now has a narrow live runtime slice for traced compiler fact,
  framework-adapter, and dynamic-boundary hover paths. Legacy hover remains the
  fallback when compiler facts are absent, stale, or only legacy-equivalent.
- Diagnostics now have a narrow live cutover for high-confidence imported and
  generated visible-symbol facts. Ambiguous, low-confidence, and
  dynamic-boundary cases remain fallback or blocked instead of being silently
  suppressed.
- Rename and safe-delete now have boundary-shadowed proof for exact static
  allow decisions plus dynamic-boundary, stale compiler fact, low-confidence,
  and generated-member blockers. Runtime blocker UX receipts compare live
  rename / symbol safe-delete request paths with compiler plans for exact
  static, dynamic-boundary, generated-member, stale-fact, and low-confidence
  cases. Mojolicious scenario 35 adds a real-workspace rename unsafe-edit
  receipt for exact local lexical edits, generated/dynamic no-edit boundaries,
  and open-document freshness without broadening live refactor behavior.
  Dancer2 scenario 37 adds a second real-workspace unsafe-edit receipt and
  proves generated `has` accessors return no rename edits. Rename also has a
  narrow live same-file lexical slice for sigiled variables when the current
  document proves exactly one `my` or `state` declaration edit. Broad
  compiler-backed, package-wide, generated, dynamic, stale, and low-confidence
  rename remain blocked or fallback/shadow data. A scoped package/compiler-backed
  pilot proof now classifies source-backed definition/reference plans as
  evidence that still requires live-path materialization guards, the runtime
  package-pilot receipt closes the real-workspace empty-plan boundary with a
  source-backed definition edit, imported-symbol blockers remain no-edit proof,
  and the compiler-allowed preview plus package-local live-pilot receipts prove
  the no-edit UX, rollback/fallback, exact-edit-set guard, and
  generated/dynamic/stale/low-confidence guardrails before authorizing any
  package-local edit. The RealBaseline false-allow receipt proves a
  compiler-allowed source-backed definition plan falls back to broader
  current-source edits, preserves no-edit preview rollback, and refreshes after
  `didChange`.
- Mojolicious scenario 36 adds a real-workspace safe-delete warning receipt for
  `workspace/willDeleteFiles` when `lib/Mojolicious/Static.pm` has dependent
  workspace files. It proves file-delete warning UX only; symbol-level
  safe-delete is live only for the narrow source-backed pilot where compiler
  allow proof, exact source guard, current-source/workspace reference guards,
  workspace identity guard, and rollback proof all pass.
- Workspace symbols now have source/freshness and real-workspace quality shadow
  proof for fresh compiler facts, framework-generated candidates,
  dynamic-boundary blockers, stale compiler facts, and candidate/noise deltas.
  Non-empty queries against the ready workspace index now persist and report a
  narrow source-backed/high-confidence live trace. Source-backed generated
  framework members may appear only as explicitly labeled virtual symbols
  anchored to framework declarations, not exact generated method bodies.
  Empty-query, partial-index, open-document fallback, generated/no-source,
  stale, dynamic, and ambiguous compiler candidates remain fallback or gated.
- Document symbols now have a narrow live source-backed parser-syntax slice for
  fresh, high-confidence `ExactAst` symbols. Framework-generated/no-source,
  dynamic-boundary, stale, low-confidence, and ambiguous candidates remain
  gated or fallback-only.
- Document symbols and workspace symbols now have runtime quality receipts that
  call the live `textDocument/documentSymbol` and `workspace/symbol` handlers
  and capture live provider counts and results. Document-symbol receipts now
  include source-backed compiler symbol counts and fact-source traces; workspace
  symbol receipts now separate exact source-backed counts from labeled
  source-backed generated/framework pilot counts and gated generated/no-source,
  dynamic, stale, and fallback/noise candidates, including explicit
  generated/dynamic false-exact shadow counts and generated-pilot
  edit-freshness proof.
  Seven BDD receipt tests cover document symbols (provider field,
  source-backed live cutover, count integrity, symbol presence, shadow state,
  notes proof, unknown-URI graceful handling), and workspace-symbol receipt
  tests cover provider field, source-backed live state, labeled generated-pilot
  state, count integrity, query echo, shadow state, notes proof, empty-query,
  no-match query, and generated/dynamic/noise gating. These receipts complete
  the runtime integration proof step for both surfaces.
- The Mojolicious scenario 32 document-symbol receipt records live
  source-backed package/sub symbols, generated `has` candidate counts,
  dynamic-boundary-shaped names, LSP shape validity, missing-symbol counts, and
  edit freshness without broadening document-symbol provider behavior.
- The Mojolicious scenario 33 workspace-symbol receipt records live-provider
  query latency, repeated-query count stability, useful/noisy hits, generated
  candidate gating, dynamic-boundary-shaped names, and edit freshness without
  broadening workspace-symbol provider behavior.
- Dancer2 scenario 39 and Catalyst scenario 41 add second- and third-project
  workspace-symbol generated/dynamic/noise receipts with edit-freshness proof.
  They do not promote generated, dynamic, stale, partial-index, or
  open-document fallback workspace-symbol candidates.
- The mixed `name` runtime receipt proves source-backed exact workspace symbols
  rank ahead of labeled generated/framework pilot symbols, preserves the
  `[generated/framework]` labels, and keeps the broader generated/dynamic/noise
  expansion receipt gated.
- The false-exact/edit-freshness runtime receipt proves generated framework
  symbols stay explicitly labeled and source-anchored, keeps dynamic-boundary
  and stale compiler-fact shadow candidates gated, and proves `didChange`
  refreshes generated pilot names before the next workspace-symbol receipt.
- The generated-symbol support review keeps the workspace-symbol tier bounded
  to non-empty ready-index source-backed symbols plus labeled generated/framework
  virtual symbols anchored to framework declarations. It does not promote exact
  generated method-body locations, generated/no-source candidates, dynamic
  candidates, stale candidates, partial-index fallback, or open-document
  fallback.
- The scoped Moo predicate generated-symbol receipt adds another generated
  member class to the workspace-symbol proof while preserving the same explicit
  label, framework-declaration anchor, and gated generated/no-source,
  dynamic, stale, fallback/noise boundary.
- The generated/no-source workspace-symbol receipt records an unanchored
  runtime-installed framework method as a blocked candidate, and Modern OO
  scenario 43 now records generated/no-source candidate names with zero live
  exact promotion. This closes the immediate generated/no-source proof request
  without promoting generated/no-source workspace symbols.
- Semantic tokens now have source/freshness proof for explicit parser/HIR
  classifications, a narrow source-backed compiler-token live slice,
  generated/no-source framework blockers, dynamic-boundary blockers, stale
  compiler facts, and fallback candidates. The live slice only records acted
  provider-decision traces for compiler-backed subroutine-declaration spans
  that already match the existing live `function` token output. The combined
  unsafe-boundary shadow receipt proves generated/no-source, dynamic-boundary,
  stale, and fallback token candidates produce no semantic-token identities,
  and the broader compiler-token false-exact receipt proves a valid
  source-backed `token:method:` compiler span still does not become a token
  identity without class-specific proof.
- Semantic tokens now have runtime integration quality receipts
  (`semantic_tokens_runtime_quality_receipt`) that exercise the live
  `textDocument/semanticTokens/full` handler and capture token count, shadow
  state, a narrow compiler-backed subroutine-declaration live-pilot span match,
  a RealBaseline project-shaped fixture receipt, and a quality-proof note.
  The receipt tests confirm receipt correctness, no-live-behavior-change
  invariant, no-token-output-change invariant, token-count parity with the live
  handler, and live-output parity across synthetic, Catalyst-shaped, and
  RealBaseline receipts. Broader compiler-fact token classes remain pending
  staged cutover.
- The Catalyst package-declaration compiler-token scoped proof proves a
  source-backed `package MyApp::Controller::Root` compiler span matches exactly
  one existing live `namespace` token, authorizes only the
  `token:package_declaration:` identity class, refreshes after `didChange`, and
  emits no new semantic-token output. It does not authorize broader namespace or
  compiler-token cutover.
- The phase-block declaration scoped cutover proof proves a source-backed
  `BEGIN`/`UNITCHECK` compiler span matches exactly one existing live `macro`
  token, authorizes only the `token:phase_block_declaration:` identity class,
  refreshes after `didChange`, and emits no new semantic-token output. It does
  not authorize broader macro or compiler-token cutover.
- The Catalyst method-call compiler-token scoped proof proves a source-backed
  `$c->stash` compiler span matches exactly one existing live `method` token,
  allows only the `token:method_call:` compiler identity class, refreshes after
  `didChange`, and emits no new semantic-token output. It does not approve
  broader `token:method:` candidates or other compiler-token classes.
- The method-declaration scoped cutover proof proves a source-backed
  `method greet` compiler span matches exactly one existing live `method`
  token, allows only the `token:method_declaration:` compiler identity class,
  refreshes after `didChange`, and emits no new semantic-token output. It does
  not approve broader `token:method:` candidates or other compiler-token
  classes.
- The field-declaration scoped cutover proof proves a source-backed
  `field $name` compiler span matches exactly one existing live `variable`
  token, allows only the `token:field_declaration:` compiler identity class,
  refreshes after `didChange`, and emits no new semantic-token output. It does
  not approve broader `token:variable:` candidates or other compiler-token
  classes. The method-declaration, package-declaration, phase-block declaration,
  field-declaration, and method-call scoped proofs close scoped class cutover steps while keeping broader
  compiler-token promotion gated by no-token-output-change, false-exact,
  fallback, and edit-freshness coverage.
- The Mojolicious scenario 34 semantic-token receipt records live token counts,
  LSP 5-tuple/span validity, expected source-backed token hits,
  dynamic-boundary string non-promotion, and edit freshness without broadening
  semantic-token output.
- Dancer2 scenario 38 adds second-project semantic-token quality proof for
  package, DSL, app, typeglob-boundary, and edit-freshness token shapes without
  broadening semantic-token output.
- Catalyst scenario 42 adds project-shaped semantic-token false-exact proof for
  generated/dynamic-looking token shapes plus edit-freshness proof without
  broadening semantic-token output.
- Other provider surfaces remain trace/proof infrastructure only until their
  own cutover proof lands.

## Navigation Live Quality Dashboard

Definition and references now have a narrow live loop for source-backed,
high-confidence facts. This dashboard is the guardrail before broadening
navigation to generated, dynamic, or lower-confidence candidates.

The source of truth for current receipt counts remains
[semantic_shadow_compare.md](semantic_shadow_compare.md); this table records
which navigation slices are live, fallback-only, or blocked.

| Slice | Live status | Receipt source | Fallback / blocker rule |
| --- | --- | --- | --- |
| `definition_exact_live` | `partial live` | [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindDefinition` release-readiness receipts | Single fresh, high-confidence, source-backed `ExactAst` candidate may answer live; otherwise legacy fallback. |
| `definition_imported_live` | `partial live` | [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindDefinition` import/export receipts | Single fresh, high-confidence explicit import, default export, or export-tag candidate may answer live; ambiguous or stale import facts fall back. |
| `references_exact_live` | `partial live` | [#8828](https://github.com/EffortlessMetrics/perl-lsp/issues/8828), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindReferences` release-readiness receipts | Fresh, high-confidence, source-backed exact occurrences may answer live when declaration inclusion is off. |
| `references_imported_live` | `partial live` | [#8836](https://github.com/EffortlessMetrics/perl-lsp/issues/8836), [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462), `FindReferences` import/export receipts | Fresh, high-confidence, source-backed `ImportExportInference` or `LiteralRequireImport` occurrences may answer live when declaration inclusion is off. |
| `generated_fallback` | `fallback / shadow` | Framework-generated `FindDefinition` and `FindReferences` traces | Generated or virtual members without exact source ranges stay labeled fallback/shadow data. |
| `dynamic_blocked` | `blocked / fallback` | Dynamic-boundary navigation traces | Dynamic-boundary candidates block compiler-backed navigation and keep legacy fallback. |
| `stale_blocked` | `blocked / fallback` | Stale-fact navigation traces | Stale compiler facts cannot answer as confirmed live navigation results. |
| `ambiguous_or_low_confidence_fallback` | `fallback / shadow` | Low-confidence and ambiguous navigation traces | Low-confidence or ambiguous candidates may inform receipts but cannot drive live navigation. |

## State Definitions

Provider states are acceptance tiers, not release labels. A provider can move to
the next tier only when the evidence for that tier is committed or generated by
the relevant receipt command.

| State | Meaning | Exit gate |
| --- | --- | --- |
| `unavailable` | No provider-specific compiler-fact proof exists yet, or the surface has no owner issue. | Add an owner issue plus fixture-backed fact evidence. |
| `fixture-backed` | The fact layer has deterministic fixtures, but provider behavior is still legacy or unproven. | Add provider-specific shadow receipts with source, provenance, confidence, freshness, fallback, and dynamic-boundary state. |
| `shadowed` | Legacy and compiler-fact outcomes are compared in receipts without changing live runtime behavior. `ranked-shadowed` and `boundary-shadowed` are shadow subtypes for candidate ranking and refactor-blocker proof. | Show zero unacceptable fixture regressions, explicit stale/dynamic handling, and a scoped live-cutover plan. |
| `provenance-backed` | Runtime or receipt output can explain the fact source, provenance, confidence, and freshness for the scoped answer. | Prove fallback behavior and real-workspace quality before broadening beyond the scoped path. |
| `partial live` | One or more high-confidence fact families are live with legacy fallback and fail-closed stale/dynamic handling. | Add real-workspace quality receipts, rollback/fallback proof, and provider-specific noise or precision deltas before expanding. |
| `live` | Compiler facts are the default provider source for the scoped surface while traces, fallback, and dynamic-boundary blockers remain available. | Keep receipts fresh when fact families or provider behavior change. |
| `blocked` | Proof found a safety, freshness, noise, or precision issue that prevents live behavior. | Close the blocker with a targeted fix and rerun the provider proof lane. |

## Cutover Matrix

| Provider surface | Current state | Current source of truth | Next proof |
| --- | --- | --- | --- |
| Diagnostics | `partial live` | Existing semantic queries suppress selected dynamic false positives, plus high-confidence imported/generated visible-symbol facts; fallback diagnostics remain available | Broader false-positive / false-negative fixture receipts before any additional diagnostic families move live |
| Completion | `partial live / source-backed receiver pilot / shadowed` | Existing completion paths remain live; high-confidence imported/exported compiler visible-symbol facts can contribute live candidates with legacy fallback; the receiver-fact pilot can contribute exact method candidates only from fresh high-confidence source-backed receiver facts; semantic-shadow fixtures, the Mojolicious scenario 28 ranking receipt, and receiver pilot receipts trace generated labels, rank deltas, useful/noisy additions, exact hash-slot receiver ranking, dynamic-boundary blockers, and medium-confidence accessor/method-return fallback preservation without promoting broader generated, dynamic, low-confidence, unknown, or workspace-wide method families | Additional real-workspace receiver quality across more project shapes before any broader generated, dynamic, method, or workspace-wide live cutover |
| Hover | `partial live / provenance-backed` | Runtime hover uses compiler-fact cutover for traced compiler fact, framework-adapter, and dynamic-boundary paths when fresh workspace facts are available; legacy hover remains fallback; hover cutover/shadow code labels imported, generated, dynamic-boundary, and fallback paths with fact-source traces and source/confidence text; Mojolicious scenario 29 records project-shaped hover surfaces without behavior changes | Additional project-shape hover quality receipts before broader generated/dynamic expansion |
| Definition / goto | `partial live / ranked-shadowed` | A single fresh, high-confidence, source-backed `ExactAst`, explicit import, default export, or export-tag candidate can drive live `textDocument/definition` with legacy fallback. Generated/no-source, dynamic-boundary, low-confidence, ambiguous, stale, and broader real-workspace candidates remain traced as fallback/shadow proof. Mojolicious scenario 30 records module-resolution, exact-local, imported-symbol, and dynamic-boundary-shaped definition probes without behavior changes. | Broader generated/dynamic migration requires additional project-shape receipts and no false-exact source-location claims |
| References | `partial live / ranked-shadowed` | Fresh, high-confidence, source-backed `ExactAst`, `ImportExportInference`, or `LiteralRequireImport` occurrence references can drive live `textDocument/references` when `includeDeclaration=false`; generated/no-source, dynamic-boundary, low-confidence, ambiguous, stale, and declaration-including requests remain traced as fallback/shadow proof. Mojolicious scenario 30 records exact-local, imported-symbol, and declaration-including boundary reference probes without behavior changes. | Broader references migration requires precision/recall receipts for generated, coderef, typeglob, and declaration-including cases |
| Rename | `partial live lexical + package-local pilot / boundary-shadowed broader compiler facts` | Same-file sigiled lexical rename can use current-document scoped AST proof only when exactly one `my` or `state` declaration edit is proven; package-local compiler-backed rename can now return live edits only when the materialized semantic source-backed edit set exactly matches the workspace source/ambiguity guard; rename plan receipts still trace exact static edits, dynamic-boundary blockers, stale compiler facts, low-confidence ambiguity, runtime blocker UX notes, live-vs-compiler exact-static receipt data, Mojolicious scenario 35 and Dancer2 scenario 37 real-workspace unsafe-edit proof, the scoped lexical cutover in [#8915](https://github.com/EffortlessMetrics/perl-lsp/pull/8915), `lsp_rename_tests::test_workspace_rename_workspace_edit_rolls_back_cleanly` rollback proof, the package/compiler-backed pilot classifier plus real-workspace source-backed definition edit proof, `perl.previewPackageRename` no-edit preview UX with rollback/no-edit receipts for imported-symbol blockers, imported-call edit-noise, and compiler-allowed source-backed definition/reference pilot previews, package-local live-pilot receipts for exact compiler-backed edits plus partial-plan fallback, generated, dynamic, stale, low-confidence, and ambiguous guardrails, RealBaseline and Dancer2 false-allow/edit-freshness receipts proving compiler-allowed package-local plans still fall back to broader fresh current-source edits when source coverage requires it, and a Catalyst false-allow receipt proving compiler-allowed package-local evidence hard-refuses ambiguous project-shaped identity with zero edits before any broad compiler-backed refactor behavior | Broader package/compiler-backed rename remains deferred; keep project-shaped unsafe-edit and edit-freshness receipts fresh when rename facts change |
| Safe delete | `partial live source-backed pilot / boundary-shadowed broader facts` | `perl.safeDeleteSymbol` can return a source-backed symbol-delete WorkspaceEdit only when the compiler safe-delete plan is allowed, the live source guard resolves an exact high-confidence subroutine definition, current source and the workspace index still have zero references, the workspace identity guard accepts the request, and rollback proof restores the original text. Safe-delete receipts still trace dynamic-boundary blockers, framework-generated blockers, stale compiler facts, runtime blocker UX notes, Mojolicious scenario 36 file-delete warning UX, Dancer2 and RealBaseline symbol-level blocker/allowed request shapes, requested RealBaseline `reset` and Dancer2 `to_psgi` delete edits plus inverse rollback proof, Dancer2 `header` referenced-method refusal, cross-file workspace-reference refusal, post-`didChange` `to_psgi` current-source reference refusal, the Catalyst `get_action` ambiguous-identity false-allow blocker, the non-subroutine/package-wide source-guard blocker, and `perl.previewSafeDelete` scoped no-edit UX | Reviewed generated/no-source and dynamic blocker receipts; broader symbol-delete migration remains deferred |
| Workspace symbols | `partial live source-backed + generated-label pilot` | Existing workspace index remains the live provider source; non-empty ready-index results can answer live with source-backed/high-confidence traces; source-backed generated/framework members may answer live only as explicitly labeled virtual symbols anchored to framework declarations, not exact generated method bodies; semantic-shadow fixtures still trace fresh compiler, generated, dynamic-boundary, stale fact, and real-workspace quality candidates; runtime quality receipts capture source-backed ready-index counts/results, labeled generated-pilot counts, mixed source-backed/generated rank proof, generated/dynamic false-exact shadow proof plus generated-pilot edit-freshness proof, scoped generated-symbol cutover proof, scoped Moo predicate generated-symbol proof, generated/no-source blocker proof, and generated/no-source/dynamic/stale/fallback-noise gating; Mojolicious scenario 33 records live-provider noise, generated candidate gating, dynamic-boundary-shaped names, and edit freshness; Dancer2 scenario 39, Catalyst scenario 41, and Modern OO scenario 43 add project-shaped generated/dynamic/noise receipts; scenario 43 now also proves generated/no-source candidate names have zero live exact promotion; support review keeps generated-label behavior bounded to virtual source anchors | Additional generated/no-source project variants and explicit-label rank/noise proof before any broader generated-symbol expansion |
| Document symbols | `partial live source-backed` | Fresh, high-confidence, source-backed parser-syntax `ExactAst` symbols can drive live `textDocument/documentSymbol` results with fallback retained for astless documents and gated generated/no-source, dynamic-boundary, stale, low-confidence, and ambiguous candidates. Semantic-shadow fixtures still trace explicit syntax, generated, dynamic-boundary, and stale fact candidates; runtime quality receipts capture live provider counts/results plus source-backed compiler traces; Mojolicious scenario 32 records real-workspace symbol quality, generated candidate counts, and edit freshness. | Generated-label proof plus additional real-workspace document-symbol receipts before generated, dynamic, or broader symbol cutover |
| Semantic tokens | `partial live source-backed token slice + scoped method/package/phase-block/field/method-call/self-method-call/lexical-variable declaration/use traces` | Existing parser/token provider remains the broad live source; semantic-shadow fixtures trace parser/HIR, compiler-backed, generated/no-source, dynamic-boundary, stale, and fallback candidates, including a combined unsafe-boundary receipt that produces no semantic-token identities and a broader `token:method:` false-exact receipt; runtime quality receipts capture live token count, shadow state, no-token-output-change proof, live span invariants, synthetic, Catalyst-shaped, and RealBaseline source-backed compiler-fact subroutine-declaration classes matched to existing live `function` token output, live-output parity, edit freshness after `didChange`, the scoped method-declaration proof that allows only source-backed `token:method_declaration:` identities already matching existing live `method` tokens and refreshing after `didChange`, the scoped package-declaration proof that allows only source-backed `token:package_declaration:` identities already matching existing live `namespace` tokens and refreshing after `didChange`, the scoped phase-block declaration proof that allows only source-backed `token:phase_block_declaration:` identities already matching existing live `macro` tokens and refreshing after `didChange`, the scoped field-declaration proof that allows only source-backed `token:field_declaration:` identities already matching existing live `variable` tokens and refreshing after `didChange`, the scoped method-call proof that allows only source-backed `token:method_call:` identities already matching existing live `method` tokens and refreshing after `didChange`, the scoped self-method-call proof that allows only source-backed `token:self_method_call:` identities already matching existing live `method` tokens and refreshing after `didChange`, and live-request provider-decision traces now include matched subroutine-declaration, method-declaration, package-declaration, phase-block declaration, method-call, field-declaration, self-method-call, lexical-variable declaration, and lexical-variable use compiler-token slices only; Mojolicious scenario 34 and Dancer2 scenario 38 record project-shaped token/span validity and edit freshness; Catalyst scenario 42 records project-shaped false-exact generated/dynamic-looking token boundaries and edit freshness; support review keeps the scoped classes output-neutral and fallback-preserving | Another scoped compiler-token class proof before broader compiler-token promotion |

## Cutover Rules

- Rename note, 2026-05-17: `refactor_runtime_blocker_ux_package_local_live_pilot_blocks_real_workspace_imported_symbol_false_allow`
  records the RealBaseline imported-symbol false-allow boundary for the
  package-local live pilot. It proves `helper` returns no edits and records a
  `package_local_live_pilot_blocked` trace rather than treating an
  imported/exported fact as a package-local edit authorization.

- Rename note, 2026-05-17: `refactor_runtime_blocker_ux_package_local_live_pilot_catalyst_false_allow_blocks`
  records the Catalyst `get_action` false-allow boundary for the package-local
  live pilot. It proves the live path hard-refuses ambiguous project-shaped
  identity with zero edits rather than treating a compiler-allowed definition
  as broad package/compiler-backed rename authorization.

- Safe-delete note, 2026-05-18: `refactor_runtime_blocker_ux_safe_delete_live_pilot_catalyst_false_allow_blocks`
  records the Catalyst `get_action` false-allow boundary for the source-backed
  live pilot. It proves the live path returns zero edits when compiler-allowed
  source-backed definition evidence and rollback proof still fail the workspace
  identity guard.

- Safe-delete support review, 2026-05-18: the referenced-source, Catalyst
  false-allow, non-subroutine/package-wide source-guard, and generated/dynamic
  live-command blocker receipts keep safe-delete `partial live source-backed`
  only. They do not promote broader symbol deletion, generated/no-source,
  imported, dynamic, stale, low-confidence, ambiguous, or workspace-referenced
  symbols. The next promotion decision requires new project-shaped proof, not
  reuse of the closed generated/dynamic blocker receipt.

- Workspace-symbol note, 2026-05-18: the generated-label pilot stays bounded to
  source-backed virtual symbols anchored to framework declarations. It does not
  authorize exact generated method-body locations or generated/no-source,
  dynamic, stale, fallback, partial-index, or ambiguous candidates. The
  generated/no-source blocker receipt and Modern OO scenario 43 no-source
  assertions close the immediate proof request without broadening live
  workspace-symbol behavior.

- Completion note, 2026-05-19: `source_backed_hash_slot_receiver_uses_exact_completion_pilot`
  and `dynamic_hash_key_receiver_preserves_imported_fallback` record the first
  source-backed receiver-fact completion pilot. The allowed live class is fresh,
  high-confidence, source-backed receiver evidence that ranks exact method
  candidates above fallback. Dynamic hash keys remain fallback-preserving and do
  not become exact hash-slot receiver facts. This does not authorize broad
  generated, dynamic, unknown, low-confidence, stale, or workspace-wide method
  completion.

- Completion note, 2026-05-19: `medium_confidence_accessor_return_receiver_preserves_imported_fallback`
  and `medium_confidence_method_return_receiver_preserves_imported_fallback`
  record the fallback boundary for newly available receiver-fact substrate.
  Medium-confidence framework accessor-return and direct method-return facts do
  not authorize exact receiver completion; imported-package fallback remains
  tiered and labeled until a separate provider receipt promotes one class.

- Semantic-token note, 2026-05-18: method-declaration, package-declaration,
  phase-block declaration, field-declaration, method-call, and self-method-call
  parity have moved to scoped
  output-neutral provider traces/proofs for `token:method_declaration:`,
  `token:package_declaration:`, `token:phase_block_declaration:`,
  `token:field_declaration:`, and `token:method_call:` only. These receipts do not move class-specific
  compiler facts into live token output. Broader compiler-backed semantic
  tokens still require class-specific cutover proof.

- Do not cut a provider over just because a fact exists.
- Every provider answer that uses compiler facts should be able to identify
  source, provenance, confidence, and fallback state where relevant.
- Generated and dynamic-boundary answers must be labeled honestly.
- Provider regressions should first appear in shadow compare or scorecard
  receipts, not as live editor behavior.

## Tracking

- Provider cutover umbrella: [#8197](https://github.com/EffortlessMetrics/perl-lsp/issues/8197)
- Hover live provenance slice: [#8369](https://github.com/EffortlessMetrics/perl-lsp/issues/8369)
- Completion live visible-symbol slice: [#8374](https://github.com/EffortlessMetrics/perl-lsp/issues/8374)
- Definition/reference real-workspace quality receipts: [#8382](https://github.com/EffortlessMetrics/perl-lsp/issues/8382)
- Definition/reference runtime integration and live-provider quality receipts: [#8462](https://github.com/EffortlessMetrics/perl-lsp/issues/8462)
- Definition exact/imported live cutover lane: [#8803](https://github.com/EffortlessMetrics/perl-lsp/issues/8803)
- References exact/static live cutover lane: [#8828](https://github.com/EffortlessMetrics/perl-lsp/issues/8828)
- References imported/exported live cutover lane: [#8836](https://github.com/EffortlessMetrics/perl-lsp/issues/8836)
- Rename/safe-delete runtime blocker UX receipts: [#8464](https://github.com/EffortlessMetrics/perl-lsp/issues/8464)
- Workspace-symbol source/freshness proof: [#8353](https://github.com/EffortlessMetrics/perl-lsp/issues/8353)
- Workspace-symbol real-workspace quality receipt: [#8378](https://github.com/EffortlessMetrics/perl-lsp/issues/8378)
- Document-symbol source/freshness proof: [#8359](https://github.com/EffortlessMetrics/perl-lsp/issues/8359)
- Semantic-token source/freshness proof: [#8360](https://github.com/EffortlessMetrics/perl-lsp/issues/8360)
- Fact-source trace receipt slice: [#8305](https://github.com/EffortlessMetrics/perl-lsp/pull/8305)
- Compiler facts: [compiler_facts.md](compiler_facts.md)
- Semantic scorecard: [semantic_scorecard.md](semantic_scorecard.md)
- Semantic shadow compare: [semantic_shadow_compare.md](semantic_shadow_compare.md)
- Provider confidence matrix:
  [provider_confidence_matrix.md](provider_confidence_matrix.md)
- Real-workspace baseline tracker: [#7949](https://github.com/EffortlessMetrics/perl-lsp/issues/7949)
