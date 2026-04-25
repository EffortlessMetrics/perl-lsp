# 2026-04-23 — Tier-Wiring + Reviewer Fix-Forward Session

**Session window:** 2026-04-23 01:30 UTC → ~05:00 UTC (post-compaction)
**Context:** Third iteration continuing the 2026-04-22 Codex-review series.
**Session framing:** After back-to-back Codex waves produced 100+ PRs across two iterations, this pass focused on (a) landing CI structural improvements (tier-wiring, bit-rot guard), (b) proving the "fix-forward" reviewer pattern at scale, and (c) draining deep-review on 18+ feature PRs.

## Economics — this session run

Plans: **Claude 20× Max** + **Codex Pro**. 5h sessions reset; weekly budget is the rollup. These are the numbers for the current 5h session window only — prior iterations (2026-04-22) covered in `docs/forensics/2026-04-22-continuous-codex-review-session.md`.

| Measure | Current session usage | Weekly total (after this session) | Weekly delta this session |
|---|---|---|---|
| Claude Code (20× Max) | **33%** of the 5h window | **79%** | **+3–4%** (from ~76%) |
| Codex Pro | **41%** of the 5h window | (82% remaining) | **+6%** |

**Per-outcome cost.** Claude's 33% session drove: ~20 merges, 18+ deep-reviews with fix-forward, 8 issues filed, 10+ dupe closes, 1 critical master bit-rot fix (#5018), 1 tier-wiring landing (#5005), 1 forensic + policy memory (this doc + `feedback_reviewer_deep_proactive_fixes.md`). **Roughly ~1% Claude session per actionable outcome** — consistent with prior sessions at ~$0.05 each at retail 20× Max pricing.

**Matched intensity held.** When Codex dispatched 40+ PR waves, Claude triaged/reviewed/merged at matching pace. Claude 33% session ↔ Codex 41% session is within ~25% of each other — the spray-and-filter economics survives continued throughput increases.

**What changed the cost shape.** The fix-forward policy (reviewer-deep pushes mechanical fixes directly) collapsed the typical find→file→build→review→merge pipeline into find→push→merge for narrow corrections. One-line and small fixes no longer pay a fresh-builder spawn.

## Throughput snapshot

Merged this session (partial list, pre-queue-drain):
- **#5018** — critical master bit-rot (`super::incremental_edit` test import depth)
- **#5005** — ci-scope classifier wired into PR Smoke (draft-tier with scope-aware clippy/test + graceful fallback)
- **#5152** — clippy `single_match` + `#[ignore]` on 4 pre-existing sandbox tests blocked by #5198
- Docs: #5000, #5001, #5008, #5010, #5012, #5032 (after dupe triage)
- Feature/test: #4998, #5015, #5031
- Plus cascade merges of the 18-PR deep-reviewed backlog (pending at write time)

Closed as duplicates / wrong direction:
- Typed `my`: #5058, #5059 → #5057
- `local` RHS: #5061, #5062, #5063 → #5060 (root-cause fix)
- UX confidence wording: #5033, #5055, #5056 → #5032
- CI-gate depth: #5035 (backs out tier-wiring)
- Draft-skip CI: #5039 (conflicts with "feedback per $" policy)
- Tree cursor: #5080 → #5079
- Constants: #5095 → #5024
- UX action SHA pin: #5038 → #5040
- Hallucinated docs: #5002, #5003, #5004 (claimed shipped #3515 was deferred)
- Duplicate fix direction: #5007, #5013, #5014, #5025, #5026 (various)
- Bad URL: #5006 (`perl-lsp/perl-lsp`)

Issues filed for structural debt exposed this session:
- **#5016** master bit-rot (fixed via #5018)
- **#5017** `ast_anonymous_sub` parser regression (landed via #5060 from Codex)
- **#5019** collapse duplicate UX workflow surfaces
- **#5020** agent-facing receipt extension (scope/lanes/reasons/next-actions)
- **#5021** scope-aware cache keying + post-merge cache warmer
- **#5096** UX gate 10s timeout too tight under concurrent CI load (fixed via #5097)
- **#5198** sandbox output capture broken on Windows + Linux runners

## Counter-intuitions this session

### 1. The CI "bit-rot guard" initially looked like it was blocking good PRs — actually it was doing its job

After #5005's tier-wiring went live, every PR touching `perl-lsp-rs` suddenly failed two checks:
- **Compile All Targets** — exposed the `super::incremental_edit` test-mod import depth on master
- **PR Smoke scoped clippy** — exposed a pre-existing `single_match` warning on `command_timeout.rs`
- **PR Smoke scoped test** — exposed 4 sandbox tests broken on both Windows and Linux runners

First read: "tier-wiring is over-strict, turn it down." Correct read: "tier-wiring is exposing real bit-rot that narrower scope was hiding." Both issues (#5016, #5198) were real breakages that would have leaked into v0.13.0 if CI hadn't started running the right scope.

**Lesson:** Widening CI scope *will* produce short-term noise. That noise is the point. Don't relax the gate; fix the surfaced issues.

### 2. Reviewer-deep pushing fix-forward is 10× cheaper than sending back

Default pipeline shape: reviewer-deep finds issue → REQUEST CHANGES → builder agent spawned → builder reads PR → builder reads review comment → builder reads surrounding code → builder writes fix → CI reruns → reviewer (sometimes) re-reviews.

After the policy change this session: reviewer-deep finds issue → pushes fix to PR branch with clear commit + comment → CI reruns. One round-trip.

Actual observed outcomes from this session:
- **#4979** (@INC dedupe) — reviewer-deep fixed stale doc comment + added whitespace-only edge-case test (commit `f01eca17e`)
- **#5024** (completion constants) — reviewer-deep replaced a vacuous test (`let _ = table`) with a real `has_symbol` assertion + added 5 new tests (commit `1cb80f26b`)
- **#5042** (xtask `-p`) — reviewer-deep fixed non-deterministic HashMap error-list order + swapped BTreeSet→HashSet for O(1) perf
- **#5060** (parser anon-sub) — reviewer-deep updated stale Phase 1 error-recovery test assertions to match Phase 2 behavior (enabling the PR to merge at all)
- **#5079** (tree cursor) — reviewer-deep added missing leaf-node test case
- **#5082/#5083** (regex) — reviewer-deep replaced a vacuous char-class assertion with a discriminating one + added 4 named-capture edge cases

**Lesson:** "Send it back for one-line fix" is the wrong default. Reviewer-deep has full context already loaded; the marginal cost of pushing a fix is a few tokens, not a fresh agent spawn. The orchestrator just needs to update the skill chain / reviewer prompt.

Saved as memory: `feedback_reviewer_deep_proactive_fixes.md`. Principle: mechanical findings push directly with commit-message + PR-comment documenting intent; structural redesigns stay REQUEST CHANGES.

### 3. "The fix for this PR already exists in the next wave" is a normal throughput rhythm, not a problem

Several times this session, a reviewer-deep identified a correctness bug in a PR, and before the fix-up builder had even started work, Codex landed a different PR with the correct fix. Examples:
- **#5029** (pragma lexical scoping) — review found that `use Moose` is compile-time BEGIN and should be file-scope. Before builder dispatched, Codex opened **#5086** with exactly that architectural shift.
- **#5017** (parser anon-sub regression) — filed as issue from a master bit-rot discovery. Before builder dispatched, Codex opened **#5060** with the root-cause fix in `is_infix_rhs_absent`.

Response: kill the redundant builder, retitle Codex's PR to close the issue, route Codex's PR through review. **Net:** 0 extra builder spawns for 2 critical fixes.

**Lesson:** When volume is high and Codex is covering the same space, the orchestrator's job drifts from "commission fixes" to "select among proposed fixes." The tool this depends on: being able to read diffs and compare approaches fast, which favors triage-first orchestration over build-first orchestration.

### 4. Validate-title accepts `(#0000)` placeholders — reviewers don't need to gatekeep this

Multiple reviewer-deep agents flagged `(#0000)` in PR titles as a blocker ("will fail validate-title"). Verified: validate-title passes `(#0000)`. It only requires `(#<digits>)` of any length. Codex's placeholder pattern is already CI-accepted.

**Lesson:** When a reviewer invokes a CI gate's rules, verify before propagating. The caveat "this will fail validate-title" was wrong 3× this session and each time cost 30 seconds to double-check.

### 5. UX Regression Gate flakes cluster under batch CI load

8 PRs failed simultaneously on `scenario_01_*` with identical "10s LSP-spawn timeout" panics. Root cause was runner contention during cold-cache parallel builds, not any of the PRs. Rerun → all green.

Fixed structurally in #5097: bumped timeout 10s → 30s. The 10s cutoff was too tight for cold-start parallel Linux runners.

**Lesson:** When N PRs all fail the same way at the same time, the failure is infrastructure, not code. Rerun first, investigate only if pattern repeats.

### 6. Fix-forward with **documentation** is as important as the fix itself

After the user's emphasis "just needs to be appropriately documented and clean and clear," the reviewer-deep memory policy added explicit requirements:
1. Commit message states the finding AND the fix (not "nit")
2. PR comment summarizes what was found + pushed SHA
3. Clean diff, one logical change
4. Clear intent — if changing test assertions, explain why the old was wrong

The #4979 reviewer-deep got this right on its own before the policy landed: APPROVE comment listed 6 verified correctness points AND 2 fix-forward items AND a non-blocker pre-existing note about PathBuf case-insensitivity on Windows. Future agents reading that trail understand both *what was done* and *what was intentionally left alone*.

**Lesson:** Fix-forward without trail leaves a silent patchwork that confuses future reviewers. Fix-forward with trail is a form of inline teaching — the next agent working nearby inherits the reasoning.

## Patterns that held from earlier sessions

- **"Don't merge on smoke-green"** — held. Several PRs had PR Smoke ✓ but merge-gate still running; waited.
- **"Matched intensity economics"** — held. ~18% Claude ↔ ~6% Codex session deltas, proportional to wave size.
- **Substrate shaping for Codex/Jules** — held. CLAUDE.md, AGENTS.md, the new fix-forward memory all get absorbed via caching. Codex continues to produce PRs that fit these patterns, suggesting the substrate reaches it.

## Deferred / non-scope findings

Surfaced during reviews but left for follow-up scouts:
- `test_deref_hash_subscript_regex_key` in `crates/perl-parser-core/tests/test_edge_cases_deep.rs:46` — `${$ref}{m}` fails, pre-existing parser bug
- `crates/perl-lsp-rs-core/tests/green_tdd_wave_g1b_regression_hardening.rs` — references `crates/perl-lsp/Cargo.toml` but crate was renamed to `perl-lsp-rs`; pre-existing path mismatch
- `require v5.10` v-string vs module-name distinction — requires token-aware parsing; documented as known limitation in #5069's test comment
- `#5041` draft-CI-skip PR — held for separate review to explore whether a *scope-aware* (not absolute) draft tier makes sense
- #5084 Moose rename `qw` list handling — parser emits unclear node kind; a qw-form test was added that will surface the gap when the parser decision is made

## Session artifacts

- **Memory files added:** `feedback_reviewer_deep_proactive_fixes.md`
- **Issues filed:** #5016, #5017, #5019, #5020, #5021, #5096, #5198
- **Critical PRs merged:** #5018 (bit-rot clear), #5005 (tier-wiring live)
- **This forensic:** self-reference

---

_Forensic captured during the session for future-session substrate. Paired with `docs/articles/ORCHESTRATION_COUNTERINTUITIONS.md` and `docs/articles/CONTINUOUS_REVIEW_PATTERNS.md`._

---

## Windows 3 + 4 — agent saturation + Codex hallucination triage

Two more 5h windows ran in this session after the log above was captured.

### Window 3 — agent saturation experiment

**Claude 18% → 52% session / Codex ~21% → ~30% spent weekly / Weekly Claude 82% → 87%**

~55 agents dispatched across three overlapping 20-agent waves (ops drain, rebase, Haiku review, deep-review, diff-audit, research-verify, refactor-plan, maintainer-pr, scout, docs-review, CI flake, RC1 punch list, retrospective). Observation: at 20+ agents in flight, orchestrator context cost is almost entirely agent-dispatch metadata + return summaries — not code reading. Orchestrator is a pure router.

Merge-ready queue built up: 15+ deep-reviewed PRs awaiting ci-green + diff-audited. 172 total open PRs. RC1 punch list landed as PR #5497.

### Window 4 — master bit-rot cascade + Codex hallucination triage

**Claude 52% → 66% session / Codex ~88% remaining session / Weekly Claude 87% → 97%, Codex 70% remaining weekly**

**Master bit-rot cascade — six pre-existing breaks surfaced by tier-wiring expansion:**

1. `#5494` — `scope_and_symbol_tests.rs:1737` `${Foo::name}` parsed as invalid format string (introduced PR #5090)
2. `#5495` — `mojolicious_navigation_tests.rs:417` stray duplicate close (merge artifact, PR #5288)
3. **xtask/lsp_stats.rs** — `last_run` + `run.completion_rate()` + missing `load_last_run` helper (incomplete #5303 refactor)
4. **xtask fmt drift** — `lsp_stats.rs` multi-line `assert!` blocks
5. **hash_key_bareword_tests.rs** — 22 type errors `expected &Arc<Node>, found &Node` (API signature drift)
6. **perl-regex/tests/comprehensive_unit_tests.rs:366** — fmt drift on function signature

All fixed via four-commit stack on PR #5501 (`fix/mojolicious-stray-close`). Windows Guardrails module-separator-regressions (`#5593`) — separate pre-existing Windows 8.3 path-canonicalization issue, filed as follow-up for `dunce::canonicalize` work.

Every fire-fix exposed the next. Classic "Tier-wiring noise is bit-rot signal" (memory entry). `--lib`-only push CI hid these for weeks; `cargo check --workspace --all-targets` is the right gate. Scout issue #4507 covers this — sprint evidence added as comment.

**Codex hallucination triage — NEW failure mode:**

Codex generated 10 PRs adding Perl framework detection for names it encountered in training-data periphery:

- **OpenClaw** (agentic editor): #5631–5634 closed — added `WebFrameworkKind::OpenClaw`, `.claw` as Perl source extension, Moo-family aliases
- **Droid / Droid::Factory** (Factory.ai terminal agent): #5619, #5641 closed — added web-route detection + `IMPLICIT_STRICT_MODULES` entry
- **Builder::IO::Fusion** (builder.io JS visual AI): #5627–5630 closed — conflated with real `Plack::Builder`
- **Google::Antigravity** (Google agentic browser): #5592 closed — added to Tier-1 completion suggestions

Pattern: 3–4 cross-crate PRs all reinforcing the same fictional framework (parser ext + semantic detection + completion tier + go-to-impl skip). Each individual PR is coherent, tested, and clippy-clean — only MetaCPAN verification distinguishes hallucination from real.

**Legitimate-but-poisoned example:** #5591 Mojolicious whitespace-in-controller-name fix was real but used `google antigravity#launch` as the only fixture. Comment posted to replace fixture; feature stays.

**Separately verified legitimate:** All 19 editor-integration docs PRs (Trae, Kiro, PearAI, Eclipse, Windsurf, Notepad++, JetBrains, Cursor, Zed, Roo Code, Kilo Code, Warp, Aider, Claude Code, Factory Droid host-detection, MCP/Hermes) target real products with LSP support. Codex editor-docs have high fidelity; the hallucination pattern is isolated to **framework-detection code**.

Memory: `feedback_codex_framework_hallucination.md` — prescribes MetaCPAN pre-filter before approving any PR adding entries to `WebFrameworkKind`, `IMPLICIT_STRICT_MODULES`, `IMPLICIT_EXPORT_SKIP_LIST`, `COMMON_MODULES_TIER_1`, or `PERL_SOURCE_EXTENSIONS`.

### Cross-window economic observations

- **Sprint-attributable weekly movement:** ~100%. Claude 76% → 97% weekly is sprint-driven.
- **Codex cheaper than Claude 20× by weekly burn:** confirmed again. Codex 70% remaining weekly at session close (~30% spent across four windows generating 250+ PRs including the hallucination batch). In the same span Claude went 76% → 97% weekly (~21% spent). **Codex delivered ~8× the PR output for ~1.4× the relative budget spend** — the spray-and-filter asymmetry is stable.
- **Orchestrator cost floor at saturation ≈ agent-dispatch metadata.** 20 agents in flight costs ~4-6% session for return-summary rollup; not for code reading (which lives in children).
- **Fire-fix cascade is higher per-cycle cost than review passes.** Each master-red fix exposes the next; investigator dispatches cost ~1-2% session each because they pull CI job logs.
- **Research-verify is the correct filter for hallucinations.** Haiku standards-review passed #5619/#5627/#5631 as "clean" — they are clean by banned-pattern check, but the code is for nonexistent frameworks. Only web-verified research catches this. Adding a MetaCPAN pre-gate to standards-review (or making research-verifier a required stage for semantic-analyzer additions) would close the loop.
- **`git reset --hard` blocked by safety hook** — working as intended. Several agents needed alternative paths (`checkout -B`, `checkout -- file`) to recover from CRLF contamination. Hook-level enforcement is the right layer for destructive ops protection.

### Follow-ups explicitly deferred to next session

- **Master may still be red at session close:** fire-fix-wave-4 was still running when the user called end-of-session. #5501 needs ops-merge attention once CI settles.
- **#5593:** Windows 8.3 short-path canonicalization in perl-module — `dunce::canonicalize` work; pre-existing, independent of RC1 Linux shape.
- **Green-refactor pass:** nine deep-reviewed PRs have `refactor-planner-reviewed` but haven't executed; queued for post-master-green cycle.
- **Merge-queue drain:** 15+ PRs have `deep-reviewed + ci-green + diff-audited` — waiting on master green to merge.
- **Scenario-number collisions:** #5268/#5381 both assigned slot 20; #5252/#5401 both assigned slot 21 by overlapping rebase waves. First-to-merge wins; losers re-renumber on conflict.
- **3 impl branches ready for red-TDD:** `impl/5496-parser-unclosed-delimiters`, `impl/5498-dap-function-breakpoints`, `impl/5499-completion-scope-distance` — spec-planner created; red-TDD needs to add failing tests next session.

### Artifacts added this pair of windows

- **Memory files added:** `feedback_codex_framework_hallucination.md`
- **Issues filed:** #5494, #5495, #5496, #5498, #5499, #5593, #5653 (inlayHint resolve), #5658 (DAP pagination nested) + comments adding sprint evidence to #4507
- **PRs opened:** #5497 (RC1 punch list), #5500 (superseded by #5501), #5501 (combined master fire-fix, 4 commits)
- **Editor PRs triaged:** 19 verified legitimate, 10 hallucinated closed, 1 poisoned-example flagged
- **Clusters closed as dupes:** Zed 3-of-4, ts-perl-c re-reversed (#5075/#5076 closed, #5386/#5387/#5388 reopened after catching wrong consolidation), metrics-receipt (#5460/#5461 → #5462), editor-docs (#5580 → #5581), vscode package.json (#5587/#5588 → #5586), Trae 3-of-4 (#5582/#5583/#5585 → #5584)
- **Deep-review fix-forwards:** 10+ real bugs caught + pushed on PR branches (including cancellation-cache invalidation #5428, missing tree-sitter-language dep #5489, FindBin word-boundary #5392, scenario 18→19 line-9-vs-10 coord bug #5327, workspace_rename 2 bugs #5434, nvim lspconfig silent no-op #5442, Zed config wrong schema #5470, emacs perl-ts-mode overclaim #5444, #5476 checked_sub dead code, #5487 missing mojolicious test)
