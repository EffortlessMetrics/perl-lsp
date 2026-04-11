# The Underselling Pattern

**A 2026-04-11 perl-lsp forensic on the capability-story gap: 102 → 116 → ~150**

---

## TL;DR

During a single day of work on 2026-04-11, perl-lsp's public capability story moved from "102 catalogued capabilities" to "116 catalogued capabilities" — a **14% correction in one afternoon** traceable to a single PR (#4107) that catalogued 14 DAP request handlers that had been shipping for weeks but had never been added to `features.toml`. Follow-up scouting found **8 more subsystems** with similar under-representation (tracked in #4114), including a 264-test refactoring crate with zero catalog entries, a 2,839-line hover generator described as "documentation", and 7-of-14 LSP 3.18 delta features already implemented while the README still claims "LSP 3.17". A defensible honest count of what has actually shipped is roughly **130–150 capabilities** — a **25–40% gap** between what the project has and what the public story claims. This is the *opposite* of the more commonly discussed "marketing overclaim" failure mode, and for a project about to ship its v0.13.0 public alpha it was probably costing adoption signal at exactly the moment adoption signal matters most. This doc names the pattern, enumerates the session's concrete evidence, diagnoses three root causes, and proposes structural fixes.

---

## 1. The Pattern Named

**Underselling** is a drift failure mode in which the public capability story
— the README, `features.toml`, status pages, announcement drafts, marketplace
listings — systematically lags behind what the codebase has actually shipped.
The lag accumulates silently, because no one routinely audits the question
"is our public story still accurate?" against live state. Every individual
PR's drift is small. The aggregate, over months, becomes structural.

### Underselling vs. overselling

Public-facing engineering failures usually get discussed in the
*overclaim* direction. A project says it ships feature X, you try feature X,
it either doesn't exist or works worse than advertised, and trust
evaporates. The remedy is well-known: scope-honest READMEs, versioned status
pages, ratcheted baselines, measured rather than asserted metrics.

Underselling is the mirror. The project actually ships feature X, but the
public story still describes the project *without* feature X. Nobody notices,
because the people who would notice are either (a) the developers who know
the code and don't re-read the README, or (b) external readers who by
definition can't tell the difference between "not built" and "not
catalogued."

It gets treated as *the polite problem*: better than the alternative, so not
worth fixing. But it has real costs, and for a pre-alpha project trying to
build adoption signal, the cost is concentrated at the worst possible
moment.

### The concrete costs of underselling

1. **External readers make decisions on stale data.** A prospective user
   evaluating perl-lsp against competitors reads the README, sees "102
   capabilities", compares against a competitor's "350 features" list,
   forms a mental model — and never discovers that the perl-lsp number
   is 25–40% too low because the catalog hasn't kept up with the code.

2. **Contributors can't tell what's built vs. what's work.** An agent or
   human filing a scout issue for "build feature X" may actually be
   filing a scout issue for "surface feature X". The session's most
   striking example: `perl-refactoring` has 264 tests, 6,284 test lines,
   workspace-wide rename, import optimization, modernization, extract-module,
   inline-subroutine — and zero mentions in `features.toml`. Any scout
   scanning the catalog for "refactoring gaps" would see a gap that doesn't
   exist in the code.

3. **Announcement drafts miss genuine credibility wins.** The draft
   announcement for v0.13.0 (`docs/articles/drafts/PERL_DESERVES_BETTER_TOOLING.md`)
   was written with the 102 figure and the "87 LSP + 10 DAP + 5 extension"
   breakdown. The corrected framing of "87 LSP + 24 DAP + 5 extension +
   2 LSP-3.18 delta features + dedicated refactoring subsystem + 13 hover
   renderers + 6 specialized completion sources" tells a materially
   different story about how production-grade the project is.

4. **Scorecard work gets mis-sized.** "Add DAP support for X" is a large
   task. "Catalog the existing DAP handler for X" is a 10-line TOML edit.
   Underselling makes the second task look like the first.

---

## 2. The Session's Concrete Evidence

The 2026-04-11 session began with a README scoping pass (PR #4046) that
framed the capability count honestly but did not audit whether the count
itself was accurate. Three separate investigations within the same day
produced the following evidence.

### 2a. The anchor: #4107 — DAP catalog undercount (102 → 116)

A research-verifier agent investigating the DAP metric scorecard sub-issue
(#4069) for the umbrella metric-stack design (#4062) cross-referenced
`features.toml` against `crates/perl-dap/src/debug_adapter/dispatch.rs`
and found a straightforward undercount:

- `features.toml` at the start of the session listed **10 DAP features**
- `dispatch.rs` match statement in the worktree worktree at
  `crates/perl-dap/src/debug_adapter/dispatch.rs:51-91` implemented
  **match arms for 37 distinct DAP request commands**
- Of those, **24 represented distinct, user-visible DAP capabilities**
  (some match arms are sub-variants of the same feature, e.g. the three
  flavours of `setBreakpoints` / `setFunctionBreakpoints` /
  `setExceptionBreakpoints`)
- **14 of those 24 capabilities were uncatalogued** — shipping code,
  shipping tests, shipping to users via the VS Code extension, absent
  from every public-facing capability document

[PR #4107](https://github.com/EffortlessMetrics/perl-lsp/pull/4107)
catalogued the 14 missing entries. The capability total moved from
**102 → 116** in a single commit that touched four files (+148/-8 lines),
all of which were pure metadata:

| Feature ID | DAP Handler | Capability |
|---|---|---|
| `dap.threads` | `threads` | Thread listing |
| `dap.pause` | `pause` | Pause running session |
| `dap.breakpoints.function` | `setFunctionBreakpoints` | Function breakpoints |
| `dap.breakpoint_locations` | `breakpointLocations` | Valid breakpoint query |
| `dap.source` | `source` | Source retrieval by reference |
| `dap.loaded_sources` | `loadedSources` | List loaded sources |
| `dap.restart` | `restart` | Restart session |
| `dap.set_expression` | `setExpression` | Assign to expression |
| `dap.cancel` | `cancel` | Cancel pending request |
| `dap.step_in_targets` | `stepInTargets` | Step-in target list |
| `dap.goto_targets` | `gotoTargets` | Goto target list |
| `dap.goto` | `goto` | Jump to target |
| `dap.restart_frame` | `restartFrame` | Restart from stack frame |
| `dap.terminate_threads` | `terminateThreads` | Terminate threads |

**The discovery path matters.** The gap was not found by a catalog audit,
a status-update cron job, or any automated check. It was found
*accidentally*, as a side effect of a research agent verifying the DAP
scorecard design in #4069. Without that unrelated investigation, the
14-handler gap would still be there.

**The undercount magnitude on the DAP subsystem alone:** 10 catalogued
out of 24 implemented = **58% undercount** on the subsystem, 14% on the
project total.

### 2b. The substrate sweep: #4114 — 8 more undersold subsystems

A scout agent (a04275492) following up on #4107 performed the audit that
#4107 itself had implicitly recommended: "if DAP was undersold by 14/24,
what else is undersold?" The sweep produced
[issue #4114](https://github.com/EffortlessMetrics/perl-lsp/issues/4114),
which enumerates **8 substantive subsystems** with production-grade test
coverage that `features.toml` either omits entirely or describes
generically. The findings, with verified counts from the live worktree:

#### 1. `perl-refactoring` crate — **zero catalog entries**

Verified from `crates/perl-refactoring/tests/`:
- **264 `#[test]` functions** across 8 integration test files
  (comprehensive_unit_tests.rs, comprehensive_unit_tests_v2.rs,
  edge_case_coverage.rs, rename_extract_coverage.rs, scoped_rename_integration.rs,
  subroutine_inline_tests.rs, workspace_rename_tests.rs, bdd_refactoring_workflows.rs)
- **6,284 total test lines** in `tests/`
- Covers: workspace-wide rename, import optimization, modernization,
  extract-module, inline-subroutine, scoped rename, workspace refactor
- **Zero mentions of "refactoring" in `features.toml`** (grep returns no
  matches on either `refactoring` or `refactor`, case-insensitive)

This is the single largest undersell of the session. A full crate with
264 tests, 4 distinct refactor categories, and workspace-wide scope
does not appear in the capability story at all.

#### 2. Hover — 2,839 lines, generic "documentation" catalog entry

Verified from `crates/perl-lsp/src/runtime/language/hover.rs`:
- **2,839 lines** of hover-generation logic
- **41 functions** (`fn` definitions) in the file
- Specialized renderers for POD markdown, XS::Typemap integration,
  Moo attributes, inherited method resolution, method dispatch chains
- `features.toml` lines 22-28 describe the feature generically as "hover
  documentation" with no enumeration of what makes perl-lsp's hover
  distinct from every other LSP server's hover

The scout issue estimated "13 distinct generation functions" as the
visible surface. The actual file has 41 functions; 13 is a conservative
floor on the user-visible distinct capabilities.

#### 3. Completion — 18 specialized source files, catalog hides them behind "150+ built-in functions"

Verified from `crates/perl-lsp-completion/src/completion/`:
- **18 source files** covering: auto_import, builtins, context, file_path,
  functions, items, keywords, methods, packages, regex_patterns,
  scope_distance, snippets, sort, test_more, variables, workspace, xs_api
- Plus `crates/perl-lsp-completion-item/src/lib.rs` as the item-shape
  contract
- Notable specialized sources: DBI type inference via `methods.rs`,
  Moo option-keys, file-path completion, XS API completion, test_more
  helper completion
- `features.toml` lines 13-19 describe completion generically

The scout issue cited "6+ specialized sources"; the actual completion
directory has 18 files implementing at least a dozen distinct source
strategies.

#### 4. Code actions — 9 action kinds, generic catalog

Verified from `crates/perl-lsp-code-actions/src/types.rs:48-64`:
- 9 `CodeActionKind` variants: `QuickFix`, `Refactor`, `RefactorExtract`,
  `RefactorInline`, `RefactorRewrite`, `Source`, `SourceOrganizeImports`,
  `SourceFixAll`, `SourceModernize`
- `features.toml` line 82 lists code actions as one entry with no
  enumeration of which kinds are supported

#### 5. Inlay hints — 14+ built-in functions instrumented, undocumented

Verified from `crates/perl-lsp-inlay-hints/src/inlay_hints.rs:393-424`:
- Hard-coded type inference for: `open`, `split`, `join`, `map`, `grep`,
  `sort`, `length`, `index`, `rindex`, `substr`, `pack`, `unpack`,
  `push`, `unshift`, `splice` (15 visible so far; the scout cited 14)
- `features.toml` lines 149-151 describe inlay hints without enumeration

#### 6. Semantic tokens — 23 token types + 13 modifiers

Verified from
`crates/perl-lsp-semantic-tokens/src/semantic_tokens.rs:159-209`:
- **Token types (23)**: namespace, type, class, interface, enum, enumMember,
  typeParameter, function, method, property, macro, variable, parameter,
  keyword, modifier, comment, string, number, regexp, operator, `sql_string`
  (Issue #2337), `sql_heredoc_keyword` (Issue #2059), `json_heredoc_key`
  (Issue #2059)
- **Token modifiers (13)**: declaration, definition, readonly, static,
  deprecated, abstract, async, modification, documentation, defaultLibrary,
  scalarVariable, arrayVariable, hashVariable
- The scout issue cited "15 types + 7 modifiers" — a floor drawn from a
  stale crate-local `CLAUDE.md`. The actual legend in code is 23/13.
  **The scout's audit was itself undersold by the stale crate documentation**,
  which is its own evidence of the pattern recursing.
- `features.toml` lines 247-250 list the feature without enumerating the
  legend

#### 7. Benchmarks — 20+ suites, zero catalog mentions

The scout found "20+ benchmark suites across perl-lexer, parser,
completion, navigation, workspace-index, dap" with zero `features.toml`
entries. Benchmarks are capability-adjacent (they're how you prove
performance claims), and a public alpha is the exact moment performance
claims matter.

#### 8. Workspace index / multi-root — multi-root correctness shipped, not surfaced

Multi-root workspace support (issue #3513) shipped via PR #3984 and the
issue is `CLOSED`. The README still listed "first folder wins" as a
known gap at the start of the session. The stale-index detection,
incremental reindex, and multi-root correctness are substrate capabilities
that the editor relies on silently; they are absent from the capability
story entirely.

### 2c. The LSP 3.18 delta: 2 implemented features not catalogued

Verified from `crates/perl-lsp/src/runtime/lifecycle/capabilities.rs:97`:
- The capabilities module consumes `markupMessageSupport` from
  `DiagnosticClientCapabilities`, which is an **LSP 3.18 delta feature**
- The same file implements `filters.relativePatternSupport` (also 3.18)
- Research-verifier a694b888 working on #4062 and #4069 determined
  perl-lsp implements **7 of 14** LSP 3.18 delta features (50%)
- **5 of those 7 are catalogued** and advertised in `features.toml`
- **2 are implemented but not catalogued**: `markupMessageSupport` and
  `relativePatternSupport`
- The README at the start of the session still claimed "LSP 3.17"

The honest framing would be "LSP 3.17 baseline + 7/14 LSP 3.18 delta
features, 5 of which are catalogued". That's a credibility win — perl-lsp
is on the LSP 3.18 early-adopter curve — that was hidden by a stale README
claim.

### 2d. Already-shipped issues found during scouting

Separate from the catalog undercount, the session's scout agents
discovered approximately **10 issues** that had been marked as "gaps" or
"in-flight" in the roadmap but were actually already implemented on master.
Verified issue states as of 2026-04-11:

| Issue | State | Finding |
|---|---|---|
| #4072 | `CLOSED` (via PR #4082) | Test assertion fix for semantic-analyzer method attributes — already done |
| #4073 | `CLOSED` (via PR #4081) | Windows path comparison in `is_allowlisted_prod_panic_hit` — already done |
| #4080 | `CLOSED` | `panic_prod_baseline.txt` drift — already done |
| #3513 | `CLOSED` (via PR #3984) | Multi-root workspace ("first folder wins") — already done |
| #3496 | `CLOSED` (via PR #4079) | Parser unclosed-block recovery — already done |
| #3485 | `CLOSED` (via PR #4050) | `use if CONDITION, PRAGMA` — already done |
| #3398 | `CLOSED` (via PR #4038) | `use feature 'switch'` — already done |
| #3482 | Landed via PR #4077 | Inherited/role methods in goto-def, hover, completion |
| #3522 | `OPEN` but ~30% done | Workspace-wide refactoring — partially shipped |
| #3523 | `OPEN` but 70-80% done | Cross-workspace-folder symbol navigation — largely shipped |

Each one of these was discovered by a scout verifying scope against live
state before filing a builder issue. **If the catalog had been accurate,
none of these "phantom gaps" would have needed a scout pass to refute.**
The cost of catalog drift is measured in unnecessary scout cycles.

### 2e. The session's total gap estimate

Taking the verified numbers together:

| Source of undersell | Capability units | Status |
|---|---|---|
| DAP handlers (#4107) | +14 | Fixed in-session |
| Refactoring subsystem (#4114) | ~6-10 (rename, extract-module, inline-sub, import-optimize, modernize, workspace-refactor, scoped-rename) | Tracked, not yet catalogued |
| Hover renderers (#4114) | ~13-20 (POD, XS::Typemap, Moo, inherited-method, method dispatch, …) | Tracked |
| Completion sources (#4114) | ~12 (vs. 1 generic entry) | Tracked |
| Code action kinds (#4114) | +8 (vs. 1 generic entry) | Tracked |
| Inlay hint built-ins (#4114) | +14 (vs. 1 generic entry) | Tracked |
| Semantic token types + modifiers (#4114) | +23 types + 13 modifiers (vs. 1 entry) | Tracked |
| LSP 3.18 delta | +2 uncatalogued | Not yet catalogued |
| Benchmarks (#4114) | +20 suites | Tracked |
| Multi-root / workspace substrate (#4114) | ~3-5 | Tracked |

The precise "honest count" depends on counting convention (does each
hover renderer get its own entry, or does hover count as one feature with
13 sub-capabilities?), but the range is defensible at **130–150**
capability entries under a granular-per-surface counting model, versus
the 116 currently published. That is a **12–29% further correction
beyond 116**, on top of the 14% correction already landed in #4107.

Rolled end-to-end from the session's starting story (102) to the plausible
honest count (~140): **~37% undersell on the initial public figure**.

---

## 3. Root Cause Analysis

Why does underselling accumulate silently when overclaim rarely does?
Three root causes, all structural.

### Cause 1: No forcing function at the point of change

When a developer (or agent) adds a new LSP request handler or a new DAP
command, the code and tests are the only artifacts that CI cares about.
`features.toml` and README updates are a **separate mental step** in a
different file that's easy to skip. Each individual skip is a small lie;
over many PRs, the lag accumulates.

The symmetric check for *overclaim* already exists: if someone adds a
capability to `features.toml` without implementing it, tests fail. The
test suite verifies that advertised capabilities work. There is **no
symmetric check for underselling**: if you implement a capability and
forget to catalog it, nothing flags the omission.

This is the structural asymmetry. Overclaim is policed by code-against-catalog
verification. Underclaim would need catalog-against-code verification, which
doesn't exist.

### Cause 2: `features.toml` has no automated audit

There is no merge-time check that asks "does this PR add a handler arm
that should correspond to a catalog entry?" A reviewer could spot it in
principle, but reviewers aren't chasing catalog completeness — they're
chasing correctness of the diff in front of them. The catalog can silently
go out of sync because no agent in the pipeline owns the question "is the
catalog still a faithful summary of the code?"

The DAP undercount is the perfect illustration: 14 handlers were added
across many PRs over many weeks, each PR focused on its own correctness
gate, none flagging that `features.toml` also needed an update. The gap
only surfaced when an unrelated research pass crossed the two files.

### Cause 3: The public story is hand-edited, not regenerated

This is the deepest cause and the only one with an obvious structural fix.

perl-lsp has two classes of status surface:

- **Regenerated surfaces.** `docs/project/status/parser.md`,
  `docs/project/status/lsp.md`, `docs/project/status/tests.md`,
  `docs/project/status/quality.md`, and the corpus baseline JSON are
  **regenerated from ground truth** via `just status-update` after every
  merge. These files stay current. They drift for minutes, never for
  weeks.

- **Hand-edited surfaces.** `README.md`, the announcement draft
  `docs/articles/drafts/PERL_DESERVES_BETTER_TOOLING.md`, the VS Code
  marketplace listing (`vscode-extension/README.md`), the catalog page
  `docs/articles/FEATURE_CATALOG.md`, the release notes
  `docs/project/RELEASE_NOTES_DRAFT.md`, and the pre-announcement
  checklist `docs/project/PRE_ANNOUNCEMENT_CHECKLIST.md` are **hand-written
  artifacts** that get updated when someone remembers. They drift for
  weeks and months.

The session's #4121 sweep found **seven distinct hand-edited files** all
still citing the old 102 figure (or older: `ARTICLE_OUTLINES.md` cited
"97 LSP capabilities", `FEATURE_CATALOG.md` cited "98 features / 88 LSP
+ 10 DAP" — doubly stale). Every file in the regenerated class was
current; every file in the hand-edited class was stale.

**Underselling is a regeneration-gap disease.** The files that get
regenerated stay current. The files that are hand-edited drift. The
public-facing public-alpha-critical files are disproportionately in the
hand-edited class.

---

## 4. What Got Fixed During the Session

The session produced four corrective artifacts, plus the scouting cycles
that enabled them. All four are linked from the umbrella metric-stack
issue #4062.

| PR/Issue | Purpose | Scope | Status (end of session) |
|---|---|---|---|
| [#4046](https://github.com/EffortlessMetrics/perl-lsp/pull/4046) | README scoping pass — added "what the numbers mean (and don't)" table, entry-points table, known-gaps section | `README.md` | Merged |
| [#4107](https://github.com/EffortlessMetrics/perl-lsp/pull/4107) | Catalog 14 DAP handlers missing from `features.toml`; 102 → 116 | 4 files, +148/-8 | Merged |
| [#4114](https://github.com/EffortlessMetrics/perl-lsp/issues/4114) | Tracking issue for 8 undersold substrate subsystems (refactoring, hover, completion, code-actions, inlay-hints, semantic-tokens, benchmarks, workspace) | Tracking only; Stage 1 builder spec attached | Open, Stage 1 in flight |
| [#4121](https://github.com/EffortlessMetrics/perl-lsp/issues/4121) | Propagate the 102 → 116 correction across 7 stale hand-edited files, rewrite stale known-gaps bullets | README, status/index.md, RELEASE_NOTES_DRAFT, announcement draft, FEATURE_CATALOG, PRE_ANNOUNCEMENT_CHECKLIST, ARTICLE_OUTLINES | Open |

Notable: #4107 was a pure metadata PR — no code changes, no new tests,
no behavior change. It was the smallest possible PR that corrected the
biggest single undercount found in the session. The ratio of impact to
diff size is the clearest argument for why underselling is cheap to fix
once it's named.

Equally notable: #4114 was filed rather than fixed. The tracking issue
splits the work into Stage 1 (refactoring, hover, completion,
code-actions — the v0.13.0 critical path) and Stage 2 (inlay-hints,
semantic-tokens, benchmarks, workspace metrics — post-alpha). The
session made a conscious scope decision not to fix everything at once,
because the v0.13.0 release schedule constrains how much catalog churn
is safe to absorb before the alpha ships.

---

## 5. Structural Fixes Going Forward

### Near-term (next session or two)

1. **Same-commit rule.** Every PR that adds a new LSP or DAP request
   handler must also add the corresponding `features.toml` entry in the
   same commit. Enforced by a reviewer check, eventually by a PR-level
   CI gate.

2. **Automated catalog-against-code audit.** A CI check that parses
   `crates/perl-dap/src/debug_adapter/dispatch.rs` and the LSP request
   dispatcher, extracts the list of handled commands, diffs against
   `features.toml`, and fails the build if there's a delta in either
   direction (overclaim or underclaim). This is the single highest-ROI
   structural fix: it would have prevented the entire 14-handler
   undercount that motivated #4107.

3. **Regenerate the README capability count.** Instead of hand-editing
   "102 catalogued capabilities" in `README.md`, derive the number from
   `features.toml` at build time (e.g. a `just readme-refresh` target
   that substitutes the count via a templated include). Same treatment
   for the announcement draft and marketplace listing.

### Longer-term

1. **Apply the ratchet model (#4105).** The capability count becomes a
   **floor metric** that can only go up. This prevents regression
   (someone deletes a feature without deleting the catalog entry) and
   makes the count a ratcheted public commitment.

2. **Layer the scorecards (#4062).** The metric-stack umbrella proposes
   per-subsystem scorecards with four rows each: coverage, correctness,
   real-user behavior, latency/cost. Underselling is *detectable* in the
   coverage row going stale relative to the code — the scorecard model
   makes the drift visible by construction.

3. **Ground-truth metrics framework (#4106).** The `cargo xtask metrics`
   framework eventually provides queries that the README and status
   pages are *generated from*. Once generated, the hand-edit class
   collapses into the regenerated class, and Cause 3 goes away.

### What this does not fix

The structural fixes address the drift, not the habit. Agents and humans
will still default to thinking "add feature X, test feature X, ship
feature X" without the catalog step. The fixes above make the catalog
step *mandatory at the pipeline level*, which is the only durable
remedy. Relying on authorial discipline is exactly how we got the 14% gap.

---

## 6. Why This Matters for Alpha Credibility

For a project one point-release away from a public alpha announcement,
the difference between:

> perl-lsp is a new Perl language server with **87 LSP features,
> 10 DAP features, and 5 VS Code extension features**.

and:

> perl-lsp ships **87 LSP features, 24 DAP features, 5 extension features,
> 2 LSP-3.18 delta features, a dedicated workspace-wide refactoring
> subsystem (264 integration tests), a 2,839-line hover generator with
> specialized renderers for POD, XS::Typemap, Moo attributes, and
> inherited-method resolution, and a completion engine with 18
> specialized sources including DBI type inference and XS API completion**.

… is the difference between *"looks interesting, might be worth trying"*
and *"this is clearly production-grade"*. The underselling wasn't just
a polite understatement. It was a concrete credibility ceiling that
perl-lsp was putting on itself, at the precise moment when every unit of
credibility matters most.

The v0.13.0 public alpha will be judged by external Perl developers and
LSP community members who have never read the code. Their entire mental
model of "what perl-lsp is" will be formed from the README, the release
notes, and the announcement blog post. Every one of those surfaces was
stale at the start of 2026-04-11. Every one of them cited either the
102 figure (which was already 14% too low) or an even older figure
(97, 98) that had been stale for longer.

The session's corrections did not make perl-lsp more capable. The code
was already there. What the session did was **narrow the gap between
what the code does and what the public story says the code does**, from
an estimated 25–40% to an estimated 12–29%, with further narrowing
tracked in #4114.

**This matters because the alpha announcement is a one-shot event.** You
get one first impression. If the first impression is formed from stale
artifacts, no amount of later correction will recover the adoption
signal that was left on the table at launch.

---

## 7. Predictive Heuristics for Future Sessions

A contributor reading this doc should be able to predict which files in
the codebase are most likely stale in a future session. The predictive
rule is simple:

> **If the file is hand-edited and contains a number, assume it's stale.
> If the file is regenerated from a ground-truth source, assume it's
> current.**

Concretely, the files to audit first during any "is our story accurate?"
pass are:

| File | Why stale-prone |
|---|---|
| `README.md` | Hand-edited, contains capability counts and gap lists |
| `docs/articles/drafts/PERL_DESERVES_BETTER_TOOLING.md` | Hand-written announcement draft |
| `vscode-extension/README.md` | Hand-edited marketplace listing |
| `docs/articles/FEATURE_CATALOG.md` | Hand-maintained catalog narrative |
| `docs/project/RELEASE_NOTES_DRAFT.md` | Hand-edited release notes |
| `docs/project/PRE_ANNOUNCEMENT_CHECKLIST.md` | Hand-maintained checklist |
| `docs/articles/ARTICLE_OUTLINES.md` | Older than README, uses pre-102 numbers |
| Per-crate `CLAUDE.md` files | Stale crate docs (e.g. the `perl-lsp-semantic-tokens` CLAUDE.md still says "15 types + 7 modifiers" when the real legend is 23 + 13) |

Files to **trust** (regenerated or machine-checked):

| File | Why current |
|---|---|
| `docs/project/status/parser.md` | Regenerated from baseline JSON |
| `docs/project/status/lsp.md` | Regenerated from `features.toml` |
| `docs/project/status/tests.md` | Regenerated from test runs |
| `docs/project/status/quality.md` | Regenerated from quality metrics |
| `features.toml` | Enforced by `cargo xtask features invariants` |
| Corpus baseline JSON | Ratcheted |

This is the operational heuristic to apply whenever external framing is
being refreshed: audit the hand-edited class, trust the regenerated class,
and file a tracking issue for every drift you find.

---

## 8. Cross-references

- **Umbrella metric stack**: [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)
  — the layered scorecard design that this pattern motivates
- **DAP scorecard sub-issue**: [#4069](https://github.com/EffortlessMetrics/perl-lsp/issues/4069)
  — where the DAP undercount was first noticed
- **Anchor PR**: [#4107](https://github.com/EffortlessMetrics/perl-lsp/pull/4107)
  — DAP catalog undercount fix, 102 → 116
- **Substrate tracking**: [#4114](https://github.com/EffortlessMetrics/perl-lsp/issues/4114)
  — 8 more undersold subsystems
- **Docs sweep**: [#4121](https://github.com/EffortlessMetrics/perl-lsp/issues/4121)
  — propagate 116 across stale hand-edited files
- **README scoping pass**: [#4046](https://github.com/EffortlessMetrics/perl-lsp/pull/4046)
  — scoped the *existing* numbers honestly; did not audit whether the
  numbers themselves were accurate
- **Ratchet model proposal**: [#4105](https://github.com/EffortlessMetrics/perl-lsp/issues/4105)
- **Metrics framework proposal**: [#4106](https://github.com/EffortlessMetrics/perl-lsp/issues/4106)

### Complementarity with other docs

- The **wisdom retrospective** for this session captures underselling as
  one of 7 session-level patterns. This doc goes deeper on the same
  pattern with specific numbers and root-cause analysis.
- The **metrics story page** (if/when filed as a contributor doc) will
  describe *how* perl-lsp reports metrics. This doc describes *why* the
  reports were inaccurate and how to prevent the recurrence.
- The `WHEN_RECEIPTS_LIE.md` article in `docs/articles/` describes
  receipts that *actively lied* (overclaim / test-helper failure). This
  doc describes receipts that *silently under-reported* (underclaim /
  catalog drift). Together they cover both failure directions of the
  receipt culture.

---

*Filed 2026-04-11 after the session that produced #4107, #4114, and
#4121. Verification performed against worktree
`.claude/worktrees/agent-aedbd0d7` at commit `36aac2cd` (origin/master
at time of writing). All cited file paths, line numbers, and counts are
reproducible from that commit.*
