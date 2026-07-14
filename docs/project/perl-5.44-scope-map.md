# Perl 5.44 Support — Scope Map

*Campaign tracking artifact. Synthesized from 6 layer ground-truth reports + adversarial verification passes. Where a verifier overturned a reader, the verifier's evidence is authoritative and is marked ⚠️ OVERTURNED.*

---

## 1. Executive Summary

The codebase is **substantially closer to 5.44 parse-acceptance than to 5.44 semantic support**, and two adversarial passes materially improved the picture. The tree-sitter grammar already models named parameters with `:`-prefix and `=`/`||=`/`//=` defaults (`grammar.js:275-294`), is deliberately order-permissive, and the vendored C snapshot is *already in sync* for these rules — so the grammar layer is at or ahead of plan assumptions. **Most importantly, the AST layer is further along than its own report claimed:** the verifier overturned two reports (AST/HIR and LSP) by showing `NodeKind::NamedParameter` on the working branch already carries `{ variable, external_name, default_operator, default_value, required }` (`crates/perl-ast/src/ast.rs:2121-2134`, populated at `variables.rs:1049-1055`, tested in `named_parameter_ast.rs`). That work is **done but uncommitted** on `claude/perl-5-44-support-sjewod-named-param-ast` — the single biggest de-risking of the campaign, and it dissolves the alleged "L-effort AST blocker" the LSP report built its sequencing on.

**True P0 blockers** (nothing works end-to-end until these land):
1. **No 5.44 version bundle** — `use v5.44` silently resolves to `BUNDLE_5_42_FEATURES` (`perl-pragma/src/version.rs:70-71`). Every feature-gate/diagnostic depends on this. *(S, leaf crate.)*
2. **Providers flatten every parameter kind** to bare `sigil+name` (`signature_help.rs:487-502`) — hover/signature-help/completion cannot surface the named/optional/slurpy data the AST now carries. *(S, provider-only — no longer AST-blocked.)*
3. **`Foreach` still holds a single scalar `variable`** (`ast.rs:1994-2003`, untouched) — multi-var / ref-alias foreach is unrepresentable; the fix is compile-breaking across ~30 match sites. *(L — sequence late.)*

Semantic 5.44 rules (named-param ordering, slurpy-hash modeling, goto-into-block with lexical nesting, XID/regex-group validation, `/xx` diagnostics) are **entirely unimplemented** — the substrate (AST nodes, PL409/PL410 label lints, version-compat PL900 lint) exists but no 5.44 rule is wired. `/xx` is a from-scratch build: regex bodies are opaque and `/xx` is actively deduped to `/x` (`hover.rs:24`).

---

## 2. Ground-Truth Per Layer

### Layer 1 — Tree-sitter grammar & corpus (`tree-sitter-perl/`)
**State:** At or ahead of plan. Named parameters, order-permissive signatures, and a "Funkier Signatures" corpus test all exist. The one real parse-acceptance gap is ref-alias foreach iteration variables. Verification: **not overturned, high confidence.**

| Claim | Verdict |
|---|---|
| `named_parameter` rule with `:` + `=`/`||=`/`//=` defaults | ✅ CONFIRMED (`grammar.js:275-282`) |
| `_signature_vars` includes `named_parameter` | ✅ CONFIRMED (`grammar.js:293`) |
| "Funkier Signatures" corpus test for `:$red, :$blue` | ✅ CONFIRMED (`test/corpus/subroutines:280-309`) |
| Signature rule does not enforce ordering | ✅ CONFIRMED (`grammar.js:297-305`, explicit comment) |
| `_for_initializer` supports only scalar / `my(...)` scalar-list, no ref alias | ✅ CONFIRMED (`grammar.js:352-355`) |
| No structural `/xx` bracketed-class support | ◑ PARTIAL — `/xx` incidentally accepted as flag token; no structural model |

**Gaps:**
- **`_for_initializer` rejects ref-alias iteration vars** (`foreach my ($k, \@v) (%hash)`) — `grammar.js:352-355` uses `paren_list_of($.scalar)`. *Blast radius:* changing `for_statement` child shape ripples to Rust for-loop lowering + semantic loop-var binding + every foreach corpus expected tree; must regenerate `src/`. **Effort M.**
- **No pinned `/xx` modifier corpus test** — acceptance is incidental. *Test-only, zero runtime.* **Effort S.**
- **Opaque regex bodies** — structural `/xx` char-class modeling is **explicitly OUT of scope** for parse-acceptance. **Effort L if pursued.**

### Layer 2 — AST/HIR (`perl-ast` NodeKind + `perl-parser-core` HIR lowering) ⚠️ REPORT OVERTURNED
**State:** NamedParameter is **already fully enriched** on the working branch; the report described committed HEAD, which has since diverged. Foreach is genuinely untouched and remains the real AST gap. Verification: **overturned=true, high confidence.**

| Claim | Report verdict | Verifier |
|---|---|---|
| NamedParameter carries ONLY `{ variable }` | CONFIRMED | ⚠️ **OVERTURNED — FALSE.** Now `{ variable, external_name: String, default_operator: Option<String>, default_value: Option<Box<Node>>, required: bool }` at `ast.rs:2121-2134`, populated at `variables.rs:1049-1055` |
| Doc-comment says "future Perl feature" | PARTIAL | ⚠️ **OVERTURNED** — replaced by "Perl 5.44 named arguments, PPC0024" (`ast.rs:2117-2120`) |
| Gap: NamedParameter needs 4 new fields; parser discards `:` name | (open work) | ⚠️ **RESOLVED, not open** — already built + tested (`named_parameter_ast.rs`) |
| first-slice: add `, ..` to ~18 arms, then enrich | (nominated) | ⚠️ **MOOT** — `git diff --stat` shows 18 files already carry `, ..` |
| Foreach carries single scalar `variable`, no Vec | CONFIRMED | ✅ **SURVIVES** — `ast.rs:1994-2003`, untouched; construction at `control_flow.rs:301,422,469` |
| HIR folds NamedParameter into shared `StorageClass::Parameter` arm | CONFIRMED | ✅ **SURVIVES** — `hir/lower.rs:1044-1057`; still genuinely open |

**Line-number drift (verifier):** NamedParameter def is `ast.rs:2121-2134` (not 2107-2111); construction `variables.rs:1049` (not 1034); Foreach `for_each_child` shifted to `ast.rs:1293` (+5).

**Remaining real gaps:**
- **Foreach `variable: Box<Node>` → `Vec`** for 5.36+ list-form / 5.44 ref-alias binders. *Blast radius:* ~30 production match sites naming `variable` without `..` (`ast.rs:548/1033/1293` serializer/visitor triplet; `workspace_index.rs:4098`; refactoring; `document_highlight`; code_actions; semantic `declaration.rs:1333`, `node_analysis.rs:399`, `symbol.rs:700`, `scope_analyzer/mod.rs:815`; `hir/lower.rs:803` `declares_iterator`) + 3 construction sites. **Effort L.**
- **HIR named-param signal split** — `record_signature_parameter` (`hir/lower.rs:1042-1058`) gives no required/external-name signal. Contained to `perl-parser-core/src/hir`. **Effort S.**

### Layer 3 — Semantic Analyzer (`perl-semantic-analyzer`, `perl-pragma`, `perl-diagnostics`, lint providers)
**State:** Implements **none** of the seven target 5.44 semantic rules. AST substrate (Goto, LabeledStatement, 4 param kinds) parses; two adjacent foundations exist — version-compat PL900 lint and flat file-scoped PL409/PL410 label lints. Verification: **not overturned, high confidence** (only coordinate corrections).

| Claim | Verdict |
|---|---|
| Named-param ordering diagnostics | ❌ REFUTED — no ordering variant in `IssueKind` (`scope_analyzer/mod.rs:69-91`, *corrected from cited :30-51*) |
| Slurpy-hash `%rest` name/value modeling | ❌ REFUTED — only unwraps to inner var |
| Duplicate call-site named-key last-wins | ❌ REFUTED — no call-site named-arg analysis exists (vacuously satisfied) |
| Feature gating for refaliasing/declared_refs/enhanced_xx | ◑ PARTIAL — PL900 machinery exists; these features absent from `FEATURE_VERSIONS` (`version_compat.rs:29-51`) |
| Version model maps `use v5.44` → features | ◑ PARTIAL — parses but caps at 5.42 bundle (`version.rs:70-71`) |
| goto-into-block with lexical nesting | ◑ PARTIAL — PL409/PL410 exist but **flat**, file-scoped; no nesting, no goto-into-block rule |
| XID identifier + regex group-name validation | ❌ REFUTED — extraction only, no validation, no diagnostic codes |

*Verifier note:* report's "grep for 5.44 empty" is inaccurate — `ast.rs:2118` contains a "Perl 5.44… PPC0024" comment. All PARTIAL/REFUTED substantive verdicts confirmed accurate.

**Gaps:** goto-into-block lexical-nesting pass (`goto_label/mod.rs`+`labels.rs`, new DiagnosticCode) **M** · named-param ordering diagnostic (`scope_constructs.rs:115-229` + new IssueKind) **M** · 5.44 bundle (see Layer 6) **S** · refaliasing/declared_refs gating (needs AST detectors) **M** · slurpy-hash + call-site named-key (needs new call-arg infra) **L** · XID/group-name validation (new codes ripple 55-code tables) **L**.

### Layer 4 — LSP feature layer (signature help / completion / hover / rename) ⚠️ REPORT OVERTURNED
**State:** Two signature-help impls (`runtime/language/hover/signature_help.rs` = live; `lsp_compat/signature_help.rs` = legacy), both **flatten** every param kind to `sigil+name` and select active param by comma-count. No named-arg key completion. The report's claim that the AST is the blocker is false. Verification: **overturned=true, high confidence.**

| Claim | Report | Verifier |
|---|---|---|
| Sig-help distinguishes named from positional | PARTIAL | ✅ survives — flattens at `signature_help.rs:487-502` |
| Sig-help reads Signature AST | CONFIRMED | ✅ survives |
| Completion suggests `name =>` keys | REFUTED | ✅ survives — none exists under `providers/completion/` |
| Hover surfaces required-vs-optional named params | PARTIAL | ✅ survives — positional-only |
| Rename handles named-param external names | PARTIAL | ◑ behavior survives, **CAUSE FALSE** — not AST-blocked; `external_name` exists |
| **AST already carries named-param data** | **REFUTED** | ⚠️ **OVERTURNED → should be CONFIRMED.** `ast.rs:2121-2134`; providers discard it via `{ variable, .. }` at `signature_help.rs:494` & `lsp_compat/signature_help.rs:230` |

⚠️ **Sequencing inversion corrected:** report's first-slice #2 (active-param mapping) and #3 (named-arg completion) claimed to be gated on "once external names exist." **They are not** — `external_name` exists now; both are **provider-only work with no AST prerequisite.** Gap #4 ("add field to NamedParameter, ~10+ arms, effort L") is **already-done work**, not pending inventory.

**Real remaining gaps (all provider-only now):**
- **`extract_signature_params` flattening** (`signature_help.rs:487-502`) — shared by hover (`hover.rs:291-303`) + core provider `param_info_from_node` (`lsp_compat/signature_help.rs:225-242`). **Effort S.**
- **Positional-only active-param** (`signature_help.rs:336-347`; `lsp_compat:381-403`) — needs key→param matching. **Effort M.**
- **No `name =>` call-site completion** — new provider under `providers/completion/`. **Effort M.**
- **Rename has no lexical-vs-external semantics** (`rename.rs:661-694`). **Effort M.**

### Layer 5 — REGEX (`perl-regex` + tree-sitter regex rules)
**State:** `perl-regex` is a shallow safety/complexity validator, not a real regex parser (its own CLAUDE.md says so). Char classes are opaque skip-regions in 3 places; `/x` has zero behavioral effect; **`/xx` is actively collapsed to `/x`** via HashSet dedup (`hover.rs:24`), asserted by `behavior_spec_tests.rs:146-160`. Verification: **not overturned, high confidence.**

| Claim | Verdict |
|---|---|
| perl-regex understands bracketed char classes | ❌ REFUTED — opaque skip (`cursor.rs:58-74`, `capture.rs:28-41,120-133`) |
| tree-sitter parses regex body structurally | ❌ REFUTED — one `regexp_content` blob (`grammar.js:1070-76,1210-22`) |
| `/x` whitespace mode handled | ❌ REFUTED — hover string only (`modifiers.rs:6`) |
| `/xx` handled at all | ❌ REFUTED — deduped to one note (`hover.rs:24`) |
| `/xx` class diagnostics detectable | ❌ REFUTED — every prerequisite absent |
| Modifiers parsed structurally | ◑ PARTIAL — flat describe-map, no flag model, no x-count |

**Gaps:** char-class content parser (new `syntax/char_class.rs`, refactor 3 skip-loops) **L** · structured modifier / `ExtendedMode {Off,Extended,Enhanced}` enum — *fix lives in `hover.rs` not `modifiers.rs`* (verifier), breaks `behavior_spec_tests.rs:146-160` **M** · `/x`-aware body tokenizer (thread modifiers through `validate()`) **M** · multi-finding diagnostic path (`validator/mod.rs`, `error.rs`) **M**.

### Layer 6 — Vendored snapshot + version/feature model
**State:** Vendored C snapshot pinned to tree-sitter CLI v0.25.9 with a documented refresh procedure; upstream grammar commit **unrecorded** (self-flagged provenance gap). Snapshot is **already in sync** for `named_parameter`/`_for_initializer` and is **benchmark-only** (only `perl-parser-comparison` depends on it) — **not a blocker**. Version model caps at 5.42. Verification: **not overturned, no corrections.**

| Claim | Verdict |
|---|---|
| Snapshot pinned to upstream ref + CLI version | ◑ PARTIAL — CLI pinned; upstream commit unrecorded (`UPSTREAM_SNAPSHOT.md:8-19`) |
| Snapshot in sync w/ grammar.js for these rules | ✅ CONFIRMED — real symbols in `parser.c:345,359` |
| Refresh procedure + validation exist | ✅ CONFIRMED |
| perl-pragma bundle high enough for 5.44 | ❌ REFUTED — caps at `BUNDLE_5_42_FEATURES` (`version.rs:67-93`) |
| Central 5.44 feature/version table to extend | ❌ REFUTED — only AST-level 5.44 mentions |
| Perl-version compatibility matrix doc exists | ❌ REFUTED — `features.toml` is LSP-capability, not Perl-version |
| Vendored snapshot blocks grammar changes | ❌ REFUTED — benchmark-only, off LSP path |

**Gaps:** `BUNDLE_5_44_FEATURES` + `>= (5,44)` branch (`version.rs:67-93,231-245`) **S** · feature-name registry 5.44 tokens (`features.rs:10-77` — silent-drop hazard if missing) **S** · new `PERL_VERSION_COMPATIBILITY.md` doc **M** · record upstream commit SHA in `UPSTREAM_SNAPSHOT.md:8-19` **S**.

---

## 3. Milestone A→E Gap Table

*Milestones inferred from the plan layers the reports were tasked against.*

| Milestone | Already exists (evidence) | Missing (evidence) | Net effort |
|---|---|---|---|
| **A. Parse-acceptance of 5.44 syntax** | Named-param grammar rule (`grammar.js:275-294`); order-permissive signatures; multi-scalar foreach; vendored C in sync (`parser.c:345,359`) | Ref-alias foreach in `_for_initializer` (`grammar.js:352-355`); pinned `/xx` corpus test | **M** |
| **B. AST/HIR representation** | ⚠️ NamedParameter **fully enriched, uncommitted** (`ast.rs:2121-2134`, `variables.rs:1049-55`, `named_parameter_ast.rs`) | Foreach → `Vec` binders (`ast.rs:1994-2003`, ~30 sites); HIR named-param signal split (`hir/lower.rs:1042-58`) | **L** (Foreach dominates) |
| **C. Version / feature model** | `parse_perl_version` accepts v5.44 (`version.rs:27-38`); PL900 version-compat lint (`version_compat.rs`); feature-name registry (`features.rs:10-77`) | `BUNDLE_5_44_FEATURES` + ladder branch; 5.44 feature tokens; refaliasing/declared_refs/enhanced_xx in `FEATURE_VERSIONS` | **S–M** |
| **D. Semantic diagnostics** | Goto/LabeledStatement AST; flat PL409/PL410 label lints; DuplicateParameter/ParameterShadowsGlobal/UnusedParameter (`scope_constructs.rs:115-229`) | Named-param ordering; goto-into-block w/ lexical nesting; slurpy-hash `%rest`; call-site last-wins; XID + regex group-name validation | **L** (cumulative) |
| **E. LSP surfacing (hover/sig-help/completion/rename)** | ⚠️ AST data present (no longer AST-blocked); live sig-help wired (`signature_help.rs:39`); hover reads signatures | Un-flatten `extract_signature_params` (`:487-502`); named active-param mapping; `name =>` completion; rename lexical-vs-external | **M** (all provider-only) |

---

## 4. Recommended First Slice

### 🎯 PRIMARY: `BUNDLE_5_44_FEATURES` + `use v5.44` version ladder in `perl-pragma`

**Files to touch:**
- `crates/perl-pragma/src/version.rs:231-245` — add `const BUNDLE_5_44_FEATURES: &[&str]` after `BUNDLE_5_42_FEATURES`.
- `crates/perl-pragma/src/version.rs:67-93` — add `version >= PerlVersion::new(5,44) => BUNDLE_5_44_FEATURES` branch above the 5.42 arm in `features_enabled_by_version`.
- `crates/perl-pragma/src/features.rs:10-77` — add any 5.44-introduced feature tokens + aliases to `known_feature_name` / `ALL_KNOWN_FEATURES` (guards the `apply_feature_state` silent-drop hazard at `features.rs:122-188`).

**Test to add (one crate):** in `crates/perl-pragma/tests/comprehensive_unit_tests.rs` (mirroring the existing 5.42 assertion at `:2095`), assert `features_enabled_by_version(PerlVersion::new(5,44))` returns the full 5.44 set and is a **strict superset** of the 5.42 bundle; add a `prop_pragma_invariants.rs`-style case at `:445` for the 5.44 ceiling.

**Acceptance criterion:** `use v5.44;` resolves to a distinct, verified-against-`perldoc.perl.org/feature` feature set (not silently `BUNDLE_5_42_FEATURES`); `cargo test -p perl-pragma` green; no consumer asserting a 5.42 ceiling regresses.

**Why it beats the alternatives:**
- **Genuinely isolated:** `enable_effective_version_semantics` (`version.rs:247`) replaces `state.features` wholesale, so the new bundle is picked up automatically — **zero downstream serializer / match-site churn**, unlike the Foreach `Vec` change (~30 exhaustive arms) or any NodeKind edit.
- **One-crate testable:** leaf crate, no cross-crate wiring; entire acceptance is `cargo test -p perl-pragma`.
- **Unblocks the most downstream work:** every feature-gating diagnostic (PL900 refaliasing/declared_refs/enhanced_xx gating in `version_compat.rs`), any "5.44-new behavior" semantic rule, and the external-truth version oracle all depend on a real 5.44 bundle existing. It is the **shared substrate** the semantic and diagnostic layers block on.
- **Forces the external-truth gate early** (CLAUDE.md requirement): bundle contents must be verified against `perldoc`, doing that research once, up front, where it is cheapest.

*One caveat baked into acceptance:* the bundle contents are a **user-visible version-gated fact** → must pass correctness review against the perldoc oracle before merge, not after.

### Runner-up #1: Un-flatten `extract_signature_params` (provider label enrichment)
`crates/perl-lsp-rs/src/runtime/language/hover/signature_help.rs:487-502` — emit `:$name` for NamedParameter, `$x = <default>` / `$x?` for OptionalParameter, keep `@rest`/`%opts` for Slurpy, so hover + signature-help surface the data the AST **already carries** (verifier-confirmed). Test: extend `lsp_signature_help_tests.rs`. Provider-only, no AST change. *Deferred to #2 only because it lives in the large `perl-lsp-rs` crate and touches shared hover/sig-help/inlay test fixtures — slightly wider blast than the leaf-crate pragma slice.*

### Runner-up #2: `ExtendedMode {Off, Extended, Enhanced}` modifier enum in `perl-regex`
Replace the HashSet dedup in `crates/perl-regex/src/analyzer/hover.rs:24` (⚠️ *the fix is in `hover.rs`, not `modifiers.rs` — verifier correction*) so `/xx` no longer collapses to `/x`; update `behavior_spec_tests.rs:146-160`. Self-contained in `perl-regex`, no char-class parser needed, unblocks all later `/xx` diagnostic work. *Narrower downstream unblock than the version bundle.*

---

## 5. Sequencing Risks

1. **Foreach `variable → Vec` is the campaign's one compile-breaking wide edit.** Unlike NamedParameter (whose additive field enrichment already landed uncommitted, using `{ variable, .. }` tolerant arms), Foreach destructures name `variable` **without `..`** at ~30 production sites, so a `Vec` change fails to compile workspace-wide until every arm is touched — including the `ast.rs:548/1033/1293` serializer/visitor triplet (must iterate the Vec) and HIR `declares_iterator` (`hir/lower.rs:803`, becomes per-binder). **Mitigation:** run a preparatory behavior-preserving PR that appends `, ..` to every Foreach arm first (the exact pattern already applied to NamedParameter per `git diff --stat`), so the semantic `Vec` change then touches only `perl-ast` + 3 construction sites. **Sequence Foreach LATE** — after A/C/E slices.

2. **NamedParameter is a solved-but-uncommitted landmine.** The enrichment (5 fields, 18 `, ..` arms, `named_parameter_ast.rs`) exists only in the working tree on `claude/perl-5-44-support-sjewod-named-param-ast`. **Risk:** any new agent re-nominating "add fields to NamedParameter" duplicates done work and may conflict. **Action:** commit/land this branch (with the HIR named-param-signal split as a clean follow-up) *before* opening provider slices — and update the AST-layer and LSP-layer reports' first-slice recommendations, which are now moot/inverted.

3. **The provider sequencing dependency asserted by the LSP report is false.** ⚠️ Named active-param mapping and `name =>` completion do **not** need to wait on an AST slice — `external_name` already exists (`ast.rs:2126`, populated `variables.rs:1051`). These can proceed in parallel with the version-bundle work.

4. **The vendored C snapshot does NOT gate grammar edits.** `tree-sitter-perl-c` is benchmark-only (sole dependent: `perl-parser-comparison`); the LSP and primary v3 recursive-descent parser do not flow through `parser.c`. A `grammar.js` edit (e.g. ref-alias `_for_initializer`) only requires regenerating `parser.c` if you want the benchmark crate to reflect it, and for `named_parameter` the snapshot is **already in sync**. **Residual risk:** the unrecorded upstream grammar commit (`UPSTREAM_SNAPSHOT.md:8-19`, bare `master` ref) weakens the audit trail if 5.44 grammar changes land upstream later — record the resolved SHA opportunistically, but it is not on the critical path.

5. **External-truth gate applies to two slices.** The 5.44 feature bundle (Layer 6) and any version-gated behavior claim are **user-visible facts** — per CLAUDE.md they require correctness review against `perldoc` *before* merge, since an AI producer can fabricate a feature list and write a test confirming it (the #3118 failure mode). Name the oracle in the PR.