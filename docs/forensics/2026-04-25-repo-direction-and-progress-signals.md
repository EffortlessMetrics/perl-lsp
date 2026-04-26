# 2026-04-25 — Repo Direction & Progress Signals

**Lens**: What the queue, commit log, and cluster patterns reveal about **what the project is becoming** — distinct from session economics, process meta-learnings, or operational anatomy
**Window**: Inferred from the 2026-04-23 → 2026-04-25 active arc + the prior week of memory entries
**Audience**: Anyone planning v0.13.0 work or making architectural decisions

This is the missing fourth lens: not "what happened in the session" (#6757), not "what patterns govern the orchestration" (#6761), not "how the waves were shaped" (#6763), but **what the repo is actually heading toward** — judged by what flows through the pipeline and what stays stuck.

---

## The big picture: v0.13.0 is closer than the issue tracker suggests

CLAUDE.md states the v0.13.0 target as a 135→30 microcrate collapse + clean-break release. The active arc shows substantial progress on this:

- **perl-symbol** crate confirmed architecturally clean (zero parser coupling, ADR-0041 invariants intact per Saturday's architectural review)
- **Wave 1 perl-module collapse** merged via #4422
- **Wave A perl-workspace collapse** in flight via #4426
- **perl-incremental-parsing** being collapsed into a thin re-export shim (-6751 lines via #5960 — kept as keeper of cluster C9)
- **perl-token** crate being hardened with checked spans, scorecard, and predicates (#6411, #6396, #6432, #6428 cluster)
- **tree-sitter-perl-rs** establishing as the v3 facade with ts-style ergonomics (memory: `project_tree_sitter_split_framing.md`)

**Implication**: v0.13.0 is plausibly 4-8 active sessions away if the collapse waves continue at current cadence (1-2 waves per week). The bottleneck is no longer "missing features" — it's "queue management for the collapse churn".

---

## What the merge stream reveals about active focus

Of the 14 PRs merged this session and the ~25-40 merged Friday, the dominant categories were:

| Category | Estimated share | What this signals |
|---|---|---|
| **Test infrastructure** (proptest, snapshots, fuzz, BDD) | ~35% | Project is in "lock down current behavior" phase, not "add new behavior" phase |
| **Parser closeout** (corpus ratchet, accuracy baseline, classification) | ~20% | Parser approaching maturity; rate of new parser bugs found is slowing |
| **Editor docs** (22+ editors covered) | ~15% | Multi-editor reach has become a project-defining axis |
| **AI completion provider expansion** (Codex Desktop, Codex CLI, Gemini, Aider) | ~10% | AI completion is a first-class feature, not an optional plugin |
| **Security fix-forward** (5 in one session: symlink, URI host, UTF-8, POD markdown, willRenameFiles) | ~10% | Active threat-modeling cadence, possibly fuzz-driven |
| **Perl-symbol surface projection** (Phase 1 SymbolRef, parity bank, cursor unification) | ~5% | Actively building the v0.13.0 symbol layer |
| **Perl-DAP perf** (real caching for stackTrace/variables/children/paging) | ~5% | DAP is being optimized for live-session use |

**Inferred direction**: the project is shifting from **"build the LSP"** to **"polish the LSP for release + reach maximum editor surface area"**. The proptest/snapshot/fuzz heavy weighting is the tell — that's what a near-release codebase looks like.

---

## What the queue reveals about pressure points

Of the 327 open PRs at session end, the bucket distribution tells a different story:

| Bucket | Count | What this suggests |
|---|---|---|
| **diff-audited awaiting CI** | 129 | Pipeline depth is healthy — review is keeping up with creation |
| **needs-builder-fix** (mostly hard merge conflicts post-rebuild) | 13 | Master root rebuild created a tail of unrebasable PRs |
| **needs-ci-fix** | 19 | Mix of stale-base + per-PR fmt drift; not a master cascade |
| **needs-diff-fix** | 10 | Mostly cross-PR `.hermes/` contamination + agent receipts |
| **merge-ready** at end of session | 3 (after final ops drain) | Throughput-bound, not signal-bound |
| **Open with no labels yet** (raw inflow) | ~150 | Codex burst inventory waiting for triage |

**Implication**: the choke point is not review depth (sufficient) and not signal quality (good). It's **parallel merge throughput** — only 3-5 PRs per ops batch can land safely without master cascade, so the queue drains slower than reviewers approve.

This suggests v0.13.0 prep should include a **CI capacity / merge throughput scaling** initiative — separate concern from feature work.

---

## The editor-support scaling problem

Across the arc, ~22 editors received setup docs:
VS Code, Trae, Neovim, Vim (coc.nvim), Emacs (perl-ts-mode), Helix, Sublime, Notepad++, GNU nano, Cursor, PearAI, Kilo Code, Codex Desktop, Codex CLI, Eclipse, Roo Code, OpenCode, Crush, Windsurf, Firebase Studio, Amazon Kiro, Claude Code.

The pattern of work is "1 PR per editor adds a setup guide + matrix entry". This has scaling problems already visible:
- **Cross-PR contamination**: #5602 and #5604 each carried byte-identical xtask gemini-alias hunks (because they were created in the same Codex thread that touched both files)
- **Destructive scope drift**: #5602 (Kilo Code) deleted 82 lines of `EDITOR_SETUP.md` without adding the promised guide; #5604 (PearAI) overwrote the Trae section
- **Order-dependent merging**: #5604 deletes Amazon Kiro entries that #5579 is the intended owner of — whichever merges second silently undoes Kiro
- **22 individual matrix entries** in `EDITOR_SETUP.md` becoming a coordination point for every new editor PR

**Direction recommendation**: the editor-support pattern needs to evolve from "1 PR per editor" to a **registry**:
- A single source-of-truth file (e.g., `editor-registry.toml` or `editors.json`) that lists each editor with: setup-doc path, install URL, language IDs, config snippet, test fixtures
- Generators that produce both the per-editor `docs/EDITORS/<NAME>_SETUP.md` and the matrix in `EDITOR_SETUP.md` from the registry
- Each "add editor X" PR then becomes a 1-3 line registry entry + auto-generated docs

This eliminates the matrix-row-collision class of cross-PR contamination and makes onboarding new editors mechanical.

---

## The perl-dap UX cluster as a lesson in batched fixes

Tracking issue #6715 was filed Saturday after the orchestrator discovered 8 perl-dap PRs (#6204, #6203, #6196, #6189, #6192, #6182, #6171, #6211) all share an identical UX scenario_01 failure: `UxHarness::new → UxClient::spawn → handshake → wait_for_response` panics with "Failed to spawn LSP server / Timeout waiting for LSP response to id=100 after 10000ms".

Each PR has its own scope (interpreter discovery cache, AST validation reuse, breakpoint sync, etc.) but they share a common upstream cause — likely a recent perl-dap change to interpreter-discovery / breakpoint-sync / framed-transport that affects LSP startup handshake.

**Per-PR fix would be wrong**: each builder would patch the same root cause from a different angle, producing 8 conflicting fixes.

**Right approach** (per #6715): one upstream fix lands, then cascade `gh pr update-branch` across all 8.

**Generalization**: when a *cluster* of unrelated PRs all fail the same downstream check on identical signature, the failure is upstream of all of them. Look for the common ancestor (typically a recent merge to master or a common dependency change), fix at the source, cascade-update.

This is now codified as `feedback_master_bit_rot_recurrence_pattern.md` in spirit but the specific *UX cluster* variant deserves its own follow-up.

---

## Architecture wins this arc

Three architecturally significant wins that are easy to miss:

### perl-symbol's clean separation
The Saturday architectural review confirmed:
- `perl-symbol` does NOT depend on `perl-parser-core` in production (only as dev-dep)
- `perl-symbol::surface` (the AST projection layer) does NOT depend on `lsp-types`
- Re-exported types (`SymbolKind`, `VarKind`) are stable enums safe for downstream consumers (`perl-semantic-analyzer`, `perl-workspace-index`)

This is the kind of architectural invariant that's load-bearing for v0.13.0's 135→30 collapse: layers can be merged when their dependencies don't cross-cut, and perl-symbol passes that test.

### The semantic façade architectural decision
Cluster C13 (semantic query façade) had 4 candidate PRs. The chosen keeper #5874 places the façade in `perl-semantic-analyzer` rather than `perl-parser`. The other 3 PRs (#5875, #5876, #5877) would have inverted the dependency direction — perl-parser gaining a new dep on perl-semantic-analyzer.

The fact that this got caught at *cluster triage* time, not at merge time, shows the verification ladder catching architectural issues before they land.

### The tree-sitter dual-facade pattern
`tree-sitter-perl-c` (conventional C bindings) coexists with new `tree-sitter-perl-rs` (v3 facade with ts-style ergonomics). Memory entry `project_tree_sitter_split_framing.md` documents this is intentional: native parser stack stays at the center, ts-perl-c is a conventional binding for ts-tooling consumers, ts-perl-rs is a new facade for Rust consumers who want ts-style API on top of v3.

This 3-layer pattern (native v3 + ts-perl-c binding + ts-perl-rs facade) is unusual and worth understanding for anyone working in this area.

---

## Security fix-forward cadence pattern

5 security fix-forwards landed in one session (#6155 symlink escape, #6156 file URI host, #6157 UTF-8 boundary panic, #6158 POD markdown injection, #6159 willRenameFiles vulnerability). The reviewer-deep agents verified attack vector coverage on each.

A 5-fix-per-session cadence suggests one of:
- Active security review or fuzz campaign upstream (someone is finding these systematically)
- Codex-thread-driven discovery as part of broader Codex bursts
- Operator's deliberate prioritization of security PRs

Whichever the cause, the project's threat model coverage is improving fast. The reviewer-deep second-opinion security review on Saturday found one OUT-OF-SCOPE follow-up (POD `E<>` entity decoding can emit raw HTML, separate from #6158's link escaping) — that follow-up should be filed as its own issue.

**Direction**: the security cadence is sustainable if the upstream discovery pipeline keeps producing findings. Worth being explicit about: is this campaign-driven or steady-state?

---

## AI completion is becoming a project differentiator

Across the arc, AI completion received material development:
- OpenAI Responses API support (#5559)
- Codex Desktop config (#5560)
- Codex CLI MCP bridge (#5679 → cherry-picked as #6716)
- Gemini CLI API key fallback (#5569)
- Gemini model alias validation (#5566)
- Aider compat (sibling editor PR)

Plus: documented in memory as `project_ai_inline_completion_design.md` which says "fully implemented and merged 2026-04-04" but needs E2E validation for release.

**Inferred positioning**: AI completion isn't an optional plugin — it's a **built-in multi-provider feature** that perl-lsp specifically supports (vs. relying on each editor's own AI integration). For a Perl LSP project, this is a meaningful differentiator: most language servers don't ship with AI provider support out of the box.

**Implication for v0.13.0 announcement**: this should be a headline feature, not a footnote.

---

## Parser is approaching "lock current accuracy" phase

The parser closeout cluster (#6230, #6232, etc.) is establishing:
- Final accuracy baseline (frozen current pass rate)
- Classification manifest (categorize remaining corpus failures by bucket)
- Performance scorecard (frozen current perf metrics)
- New blocking CI gates (`parser_corpus_ratchet`, `cpan_corpus_ratchet`)

This is "lock current behavior so we don't regress" infrastructure, not "improve current behavior". The transition from "actively fixing parser bugs" to "ratchet-locking parser accuracy" is a meaningful project lifecycle marker.

**Direction**: future parser work will be primarily defensive (don't regress the locked accuracy), not exploratory (find new bugs to fix). This changes how parser PRs should be evaluated — a PR that improves one corpus class but regresses another should be treated as breaking the ratchet, not as a trade-off.

**Caveat from #6230 deep-review**: the new blocking gates as currently registered always fail in CI (CPAN not installed, baseline from different machine). The infrastructure direction is right; the CI integration needs work before the gates can actually block bad PRs.

---

## Refactor tooling (#3522) is becoming a coherent feature

Three sub-features for issue #3522 progressed in parallel:
- Workspace-wide rename for static cross-file references (#5841, post-rebuild keeper #6053)
- Safe-delete reference preflight (#5836-5839, post-rebuild keeper #6047)
- Cross-file reference query foundation (#5829, keeper #5830)
- Plus: module-move import rewriter (#6043, orthogonal Phase 2)

These are the building blocks of **proper IDE refactoring tools** for Perl — not just rename-in-current-file but workspace-aware operations that understand cross-file references. This is feature territory that makes perl-lsp competitive with Rust Analyzer / TypeScript LSP for real refactoring work.

**Direction signal**: rename + safe-delete + module-move = the trio of refactor operations that turn "good go-to-def" into "actually usable refactoring". When these land, perl-lsp's IDE-feature pitch changes substantively.

---

## The 173+ locked worktrees observation, restated as direction

Disk-overhead-wise this is annoying but not load-bearing. **As a signal**, it's important: it represents the steady-state cost of the orchestrator-driven development model. Each session leaves N worktrees locked because agents were killed by quota limits before their wrap-up step ran.

**Direction**: as the orchestrator-driven model becomes the standard development pattern (rather than a high-volume anomaly), the worktree-cleanup discipline needs to become routine. This means:
- A `just clean-worktrees` recipe (CLAUDE.md mentions this exists — verify it's robust)
- A pre-session check that prunes pre-session worktrees from prior sessions
- A post-session step that explicitly releases unlocked worktrees rather than relying on agent wrap-up

---

## Open architectural questions for next phase

These came up in the arc but didn't get resolved:

1. **#5793 vs #5795 architecture choice for exporter metadata (#3416)**:
   - #5795 stores `ExporterMetadata` directly on `ClassModel` (co-located with per-package state)
   - #5793 introduces a dedicated `semantic::exporter_metadata` module with façade access
   - This is a real architecture choice (co-located vs. separation-of-concerns), not a quality-tier choice. Needs plan-review on issue #3416.

2. **C21 Unicode lexer cluster final architecture**:
   - Plan-reviewer recommended reopening #6098 + cherry-picking #6099's emoji tag tests
   - The defensive `normalize_char_boundary()` helper in #6097 was identified as an anti-pattern (silently drops bytes from tokens)
   - Decision: clean refactor (#6098 style) over defensive guards (#6097 style) for boundary handling

3. **The CPAN corpus integration in CI**:
   - The #6230 ratchet wants to enforce CPAN corpus pass rate but CPAN isn't installed in the runner
   - Either install CPAN in the runner image (build time cost) or skip the gate gracefully when corpus is absent (gate becomes informational)

4. **Editor-registry consolidation** (per the editor-support scaling section above): worth a dedicated proposal/RFC

5. **Memory entry consolidation**: the entry count has reached useful density (~75 entries). The parallel "memory consolidation review" agent dispatched late in the session ran out of quota before producing the consolidation proposal. Worth re-running.

---

## Synthesis

The repo is in late-pre-release shape:
- Architecture maturing (clean boundaries, façade patterns, microcrate collapse in flight)
- Parser approaching "ratchet current accuracy" phase
- LSP feature surface expanding (refactoring tools, AI completion, DAP perf)
- Multi-editor reach has become a project-defining axis (22+ editors)
- Security cadence is healthy (5 fix-forwards / session)

The **bottleneck** is not feature completeness or quality — it's **queue management for the collapse churn**. The 327 open PRs with 129 awaiting CI and 13 needing manual rebase represent operational debt from the high-velocity Codex burst pattern, not architectural debt or missed features.

For v0.13.0:
- **Headline-able features**: AI completion (multi-provider), refactoring tools (rename + safe-delete + module-move), perl-symbol clean architecture
- **Stability narrative**: parser accuracy ratchet, snapshot tests, fuzz coverage, security fix-forward cadence
- **Editor reach narrative**: 22+ editors with setup docs (recommend registry consolidation before announcing)
- **Architecture narrative**: 135→30 microcrate collapse demonstrates discipline

For the **next 1-2 sessions**:
1. Drain the 13 needs-builder-fix tail (manual cherry-pick of pre-rebuild PRs)
2. Cluster-fix the perl-dap UX issue (#6715) instead of per-PR
3. Resolve the #5793 vs #5795 architecture choice for #3416
4. C21 Unicode lexer follow-up (reopen #6098 + cherry-pick tests)
5. Editor-registry RFC if scaling problem is becoming load-bearing

For the **session-end ritual**:
- Worktree cleanup pass to manage disk overhead
- Memory consolidation pass (entries have reached useful density)
- Forensics doc to capture session-specific patterns

---

## Concrete codebase observations (from agent reports across this session)

These are crate/file/function-level observations gathered across deep-review reports, ensemble triage, and direct investigation. They're specific enough that future operators can use them as orientation when working in these areas.

### Lexer (crates/perl-lexer/src/lib.rs)

The lexer is the most-touched file in the active arc. Specific load-bearing patterns observed:

- **`current_quote_op` state machine**: set/cleared in `parse_quote_operator` (around line 3203). Stale state is harmless in practice (consumers gate on `LexerMode::ExpectDelimiter`) but is a latent hazard. PR #6091 cleared it eagerly before the early-return path.
- **`LexerMode::ExpectTerm` vs `LexerMode::ExpectOperator`**: the mode is what determines whether `/` parses as regex vs division. Word operators (and, or, xor, not), expression keywords (return, die, warn, do, eval), and bare builtins must set ExpectTerm. PR #6088 caught a regression where `return /pat/` was lexing as division.
- **EXPECT_TERM_KEYWORDS** constant introduced by #6088: sorted array searched via `binary_search`, replacing a hard-coded match arm. All keywords must be in `1..=9` length bound and present in LEXER_KEYWORDS.
- **consume_balanced_segment_in_string vs consume_balanced_segment**: the `_in_string` variant stops at outer quote `"` to prevent runaway inside double-quoted interpolation. PR #5361 was caught removing the `_in_string` variant while leaving 7 callers using it — would have silently corrupted lexing of double-quoted braces.
- **after_var_subscript** flag: set at the brace-opening sites for `${...}`, `@{...}`, `%{...}`. Drives `hash_brace_depth` tracking which in turn drives s/tr/y delimiter detection. Removing the flag (as #5361 did) breaks the chain.
- **is_quote_delim / is_paired_delim / is_fat_arrow / peek_nonspace_and_following()**: the disambiguation guards for s/tr/y. The interplay between these is subtle.
- **Hot-path pattern**: `try_identifier_or_keyword` is benchmarked and #6100 added an ASCII byte-scan fast path. Multiple lexer perf PRs target this function.

### Parser-core (crates/perl-parser-core/)

- **engine/parser/control_flow.rs**: handles typed `catch Class with { ... }` (Error.pm style). PR #6252 logic correctly distinguishes the three catch forms via mutual-exclusion guards.
- **engine/parser/statements.rs + variables.rs**: handle the `class` keyword disambiguation. PR #6255 added `peek_second()` guard to route `class("Widget")` (paren follows) to expression parsing instead of `parse_class()`.
- **syntax/error/classifier.rs**: recovery-salvage metrics + dirty classification. The `classify_dirty_file` function enforces exactly-one-category by returning `None` for files in multiple buckets.

### Parser incremental (crates/perl-parser/src/incremental/)

- **incremental_document.rs**: batch edit application + UTF-8 boundary handling. The `validated_edit_range` helper (added by #5851) is shared across all four entry points (apply_edit, apply_edits, apply_edit_to_string, apply_edit_in_place).
- **incremental_edit.rs**: `IncrementalEditSet` with `normalize_and_validate` API. The (allow_overlaps, filter_no_ops) bool params (chosen over an options struct) is a deliberate API simplicity call.
- **incremental_checkpoint.rs**: `invalidate_range` does token-granular splitting (per #6001). Has a documented coordinate-space invariant: `adjust_positions` shifts segment boundaries to post-edit space but leaves token byte offsets in pre-edit space. The doc was added in this session as fix-forward by reviewer-deep agent.
- **perl-incremental-parsing crate**: being collapsed into a thin re-export shim of `perl_parser::incremental` (-6751 lines via #5960). README documents source-of-truth ownership.

### Parser refactor (crates/perl-parser/src/refactor/workspace_refactor.rs)

- **find_package_at_offset (correct)** vs **find_package_declaration (broken in some paths)**: the former uses last-wins-before-offset semantics (correct for "what package am I in at this byte position"). The latter uses `text.lines().find_map()` which returns the first package declaration in the file. PR #5367 was caught using the wrong version, which would have edited the wrong package's variables in multi-package files.

### Perl-symbol architectural keystone (crates/perl-symbol/)

- **Confirmed zero parser coupling**: production deps are perl-ast + serde only. perl-parser-core is dev-dep only.
- **Module structure**: types/ (SymbolKind, VarKind enums), cursor/ (byte-oriented extraction), surface/ (AST projection), index/ (trie + inverted index).
- **SymbolKind variants**: Package, Class, Role, Subroutine, Method, Variable(VarKind), Constant, Import, Export, Label, Format. Stable enum safe for downstream re-export.
- **SymbolDecl fields**: kind, name, qualified_name, full_span, anchor_span, container, declarator. The canonical AST projection record.
- **extract_symbol_decls(root, current_package)** returns `Vec<SymbolDecl>`: the canonical projection function. Walks AST nodes in surface/decl.rs. Lives at the architectural choke point between parser AST and IDE concepts.
- **cursor/mod.rs**: post-#6375 unified into byte-oriented `token_span_at_byte` helper with `ScanOptions` struct. Replaces three separate character-based functions.

### Workspace index (crates/perl-workspace-index/src/workspace/workspace_index.rs)

- **Cross-package bare-name reference storage**: when `package Bar` calls `process_data()` (unqualified), the indexer stores the ref as `"process_data"` (bare) AND `"Bar::process_data"` (qualified with current package). It does NOT store it as `"Foo::process_data"` even if Foo::process_data is the resolved target. PR #6053 was caught misunderstanding this — its `is_ambiguous_sub_reference` was dead code because `find_refs(key="Foo::process_data")` would never return refs for Bar.pm's call.
- **IndexAccessMode::Partial**: degraded-index mode where some operations propagate refusal as JSON-RPC error, others fall through to same-file. The behavioral asymmetry between Ok(empty) and Err(refusal) in Partial mode is a design choice worth understanding.

### Perl-lsp-rs (crates/perl-lsp-rs/)

- **src/runtime/language/completion.rs** lines around 527 and 745: the sortText serialization sites. PR #6447 needs an 8-line patch here (saved to /tmp during cleanup; see #6447 for context).
- **src/runtime/dispatch/lifecycle.rs**: set_trace dispatch + shutdown flow. PR #5946 added proptest state machine for trace level lifecycle. PR #5938 has duplicate `#[test]` attribute (syntax error caught by standards review).
- **src/runtime/workspace.rs**: willRenameFiles handling. PR #6159 (security fix) closed the closed-file-read vulnerability. Workspace folder URI handling (string vs object form) in `extract_workspace_folder_uris` — PR #5544 added defensive both-form acceptance.
- **src/security/sandbox.rs**: the file with persistent CRLF noise in main checkout. Restored multiple times this session.

### Perl-lsp-rs-core (crates/perl-lsp-rs-core/)

- **src/providers/completion/completion/variables.rs**: sort_text uses `"1{dist_key}_{name}"` format. The `{dist_key}` comes from `ScopeDistance::sort_key()` with `{:02}` padding (so `b09` < `b10`). Verified correct in #5881 deep-review.
- **src/providers/completion/completion/scope_distance.rs**: `ScopeDistance` enum with Immediate/Parent/Global variants, `parent_hops_to_scope` walks ancestor chain with `saturating_add(1)` and a 100-hop safety cap.
- **src/providers/code_actions/modernize.rs + src/providers/type_hierarchy/mod.rs**: have line-offset hot paths that #6446 quadratic-fix touched. Same files appeared as "reformatted" in PR Smoke failures because xtask fmt re-formatted them after the fix landed.
- **src/tooling/perl_critic/built_in.rs**: hosts the policy registry. PRs #5710 (ProhibitStringyEval) and #5711 (ProhibitTwoArgOpen) both touch this file but conflict with #5712 (bareword filehandle policy already merged). Cherry-pick guidance for the unmergeable cluster: drop the bareword piece, keep the new policy.

### Perl-dap (crates/perl-dap/)

- **src/debug_adapter/mod.rs**: hosts `VariableCache` struct (PR #6192 added).
- **src/debug_adapter/variables.rs**: `handle_variables` with caching layer. Pagination via `get_page(ref, start, count)` memoizes page slices keyed by (start, count). Pre-warming on upsert.
- **src/debug_adapter/execution.rs**: 6 transition points where `variable_cache.clear()` is called. PR #6192 added the 6th (interrupt/pause handler).
- **src/inline_values.rs**: dedup via `HashSet<(usize, String)>` keyed on (line_idx, var_name). `is_special_variable_name` + `SPECIAL_VARS` are shared constants.
- **AssignmentOpTokensRe regex** (added by #6196): tokenizes operators to avoid `==` `!=` `<=` `>=` `<=>` false positives in expression validation.

### Perl-lsp-config (crates/perl-lsp-config/src/lib.rs)

- **The 21-line security fix** (#6051): removed `perlPath`/`perlArgs` handling from `WorkspaceConfig::update_from_value`. Closed the ACE vector where untrusted `workspace/configuration` responses could override Perl executable used for @INC probing.
- **Defaults**: `perl_path = None`, `perl_args = vec![]`. Causes `fetch_perl_inc` to use system perl discovery (the trusted path).

### UX testing (crates/perl-lsp-ux-tests/)

- **tests/ux_scenario_01_simple_file.rs**: the test that 8 perl-dap PRs all timeout on. Panic at line 30/46/81 is `UxHarness::new(ScenarioConfig::default()).expect("Failed to create UX harness")`.
- **src/lib.rs** lines 157-175: `UxHarness::new` calls `UxClient::spawn` at line 171-172. The hang is in the LSP server initialize handshake.
- **src/lib.rs:560**: `resolve_binary` succeeds (gated by `binary_available()`). The hang is post-binary, in handshake.
- **fixtures/editor_ux_fixture_matrix.json**: 22 workflow entries (master). PR #5403 was caught with a stale matrix (19 workflows) — would have failed `editor_ux_fixture_matrix_covers_all_scenarios` test post-merge.

### CI infrastructure (xtask/, .ci/)

- **xtask/src/tasks/cpan_corpus.rs:604**: hard `return Err(...)` when `target/cpan-corpus/lib/perl5` doesn't exist. PR #6230's blocking gate fails because CI runner doesn't have CPAN installed.
- **xtask/src/tasks/corpus_audit.rs**: hosts node_kind_coverage, valid_parser_gap_count, recovery_salvage_rate ratchets. All three use `+ 1e-6` epsilon guards. PR #6230's `clean_rate` comparison was caught missing the epsilon.
- **xtask/src/tasks/parser_corpus_sweep.rs**: `SweepReport` struct now has `dirty_classification` field (added by #6232). PR #6232 was caught missing field initializers in `test_report()` helper and `update_status/parser.rs:362` test — both fixed via fix-forward.
- **xtask/src/tasks/fmt.rs:42**: aborts at first failure with misleading "Failed to format `<crate>/Cargo.toml`" message. Documented in `feedback_xtask_fmt_false_cascade.md` as the source of repeated false-master-cascade signals.
- **.ci/GATE_REGISTRY.toml + .ci/gate-policy.yaml**: gate definitions. PR #6230 added `parser_corpus_ratchet` and `cpan_corpus_ratchet` as `blocking = true` / `required: true` — currently fail in CI.

### Tree-sitter triplet

- **tree-sitter-perl/**: legacy C tree, ~7000 files. Excluded from workspace per CLAUDE.md. Retained for benchmarking only.
- **crates/tree-sitter-perl-c/src/lib.rs**: conventional Rust binding. Has `parse_perl_bytes`, `parse_perl_code`, `parse_perl_file` wrappers. PR #6138 added typed `ParsePerlError` enum (#[non_exhaustive]) with LanguageSetup/Io variants and From conversions.
- **crates/tree-sitter-perl-rs/src/lib.rs**: v3 facade with ts-style ergonomics. PR #6026 (cluster keeper) adds first semantic overlay queries (`Tree::semantic_overlay()`).

### Editor extension (vscode-extension/)

- **extension.ts** lines 1588-1601 + 1672: symlink escape prevention. `hasSafeExistingAncestor` walks up to first existing ancestor, calls `fs.realpathSync` (resolves symlinks), checks lexical containment against workspaceRealPath. Re-checked at mkdirSync to close TOCTOU race.
- **package.json**: extension manifest. Carried Perl file extension lists (.pl, .pm, .cgi, .fcgi, .PL, .t). PR #5446 broadened this. PR #5576 had the unresolved merge conflict.

### Why this matters for direction

The crate-level pattern shows the project has **load-bearing files in the 5-15 file count**. Most action is concentrated in:

1. perl-lexer/src/lib.rs (single file, repeated touches)
2. perl-parser-core/src/engine/parser/*.rs (handful of files)
3. perl-parser/src/incremental/*.rs (4 files, being collapsed)
4. perl-symbol/src/{cursor,surface,index}/mod.rs (architectural keystone)
5. perl-lsp-rs/src/runtime/{dispatch,language,workspace}/*.rs (LSP integration)
6. perl-dap/src/debug_adapter/{mod,variables,execution}.rs (DAP core)
7. xtask/src/tasks/{cpan_corpus,parser_corpus_sweep,corpus_audit}.rs (CI infrastructure)
8. vscode-extension/{extension.ts,package.json} (extension surface)

This concentration is **healthy for a v0.13.0 push**: clear ownership, well-understood files, smallish blast radius per change. A scattered "many small changes across many files" pattern would be more concerning.

**Direction implication**: when planning v0.13.0 work, weight investment toward the load-bearing files above. PRs that touch these files have higher leverage per token spent than PRs that touch peripheral files.

---

## Cross-references

This is the 4th lens of the 2026-04-25 session retrospective set:
- `2026-04-25-3day-arc-economics-and-learnings.md` (#6757) — quantitative metrics across 3 days
- `2026-04-25-process-meta-learnings.md` (#6761) — pattern-level orchestration analysis
- `2026-04-25-orchestration-anatomy.md` (#6763) — operational anatomy (waves, collisions, dialogue)
- **This doc** — repo state + direction signals

Plus prior-day forensics:
- `2026-04-22-continuous-codex-review-session.md`
- `2026-04-23-tier-wiring-reviewer-fix-forward-session.md`
- `2026-04-24-extended-throughput-session-retrospective.md`
- `2026-04-25-pr-queue-drain-session.md`
- `2026-04-25-session-final-state.md`
- `2026-04-26-session-priorities.md`

Together: economics + patterns + anatomy + direction. Four lenses that together describe both *what happened* and *what to do next*.
