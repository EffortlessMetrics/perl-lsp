# 2026-04-24 Session Retrospective — Master Unblock + Codex Hallucination Triage + Layer-Diversity Triage

**Session window:** 2026-04-23 21:00 UTC → 2026-04-24 01:00 UTC (post-reset, fresh 5h + fresh weekly budget)

## Headline numbers

- **41+ PRs merged** across 10 ops drain rounds post-fire-fix
- **21+ hallucinated PRs closed** across 4 product clusters (OpenClaw / Droid / Builder.io Fusion / Google::Antigravity / Hermes / MCP-as-Mason)
- **Master unblock stack: 9 fire-fix waves** on PR #5501 (later #5724 + #5749 for post-merge residuals)
- **250+ PRs triaged** in total across the session span
- **~10% Claude weekly spent** this run; **~3% Codex weekly spent**. Spray-and-filter asymmetry holds: Codex produces ~8× output volume for ~1.4× relative budget spend.

## Principal patterns captured

### 1. Codex framework hallucination — discrete failure mode

Codex, when given broad scopes touching Perl framework detection, confidently generates PRs adding WebFrameworkKind entries, IMPLICIT_STRICT_MODULES entries, PERL_SOURCE_EXTENSIONS entries for names it encounters in its training periphery (agentic editors, JS visual builders, AI coding tools). Each hallucination produces a coherent 3-4 PR cluster (parser ext + semantic detection + completion tier + go-to-impl skip) where every individual PR is tested, clippy-clean, and plausible on reading.

**Confirmed hallucinations (all closed this session):**

| Product | PRs | What it actually is |
|---|---|---|
| OpenClaw | #5631, #5632, #5633, #5634 | Agentic editor |
| Droid / Droid::Factory | #5619, #5641 | Factory.ai terminal coding agent |
| Builder.io Fusion | #5627, #5628, #5629, #5630 | JS visual AI builder |
| Google::Antigravity | #5590, #5591, #5592, #5594 | Google's agentic dev browser |
| Hermes Agent | #5635, #5636, #5637, #5638 | Nous Research model family |
| MCP-as-Mason | #5625 | Anthropic Model Context Protocol (MCP ≠ Perl Mason's `.mc`/`.mp`/`.mi`) |

**Detection heuristic that works:** MetaCPAN verification (`curl -s "https://fastapi.metacpan.org/v1/module/_search?q=<Name>&size=3"`). Zero results plus AI-product name ≈ hallucination.

**Detection heuristic that does NOT work:** Haiku standards-review. Every hallucinated PR passes "banned patterns + title format + scope" checks because the violation is against the Perl ecosystem (nonexistent modules) not Rust coding standards. Only web-backed verification catches this class.

**Architectural prescription:** a MetaCPAN pre-gate for any PR adding entries to `WebFrameworkKind`, `IMPLICIT_STRICT_MODULES`, `IMPLICIT_EXPORT_SKIP_LIST`, `COMMON_MODULES_TIER_1`, or `PERL_SOURCE_EXTENSIONS`. See `memory/feedback_codex_framework_hallucination.md`.

**Related closed-alias pattern:** #5633 treated `OpenClaw` as a Moo-family alias. The cross-layer Codex task was patching `detect_framework` to alias a fake name to a real framework family. File-path grep (`class_model.rs` + `frameworks_moo.rs`) is a cheap high-precision detector for the whole class.

### 2. Broad Codex scopes produce layer diversity, not duplicates

When a Codex task prompt is broadly scoped ("fix encoding" / "improve completion" / "harden URI parsing"), the 4-shot outputs tend to each pick a **different slice of the stack** rather than producing overlapping implementations. Calling them "duplicates" on title match collapses intentional layer-diversity into a single PR and throws away 3 useful contributions.

**Example (encoding cluster this session, 12+ PRs):**

| Layer | PRs |
|---|---|
| `workspace.rs` file reads (**same layer**, genuine dupes) | #5740, #5741, #5743 |
| `util/mod.rs` shared decode helper | #5742 |
| `navigation.rs` LSP provider | #5742 |
| `perl-uri` URI parsing mojibake | #5738 |
| `perl-critic` tool output mojibake | #5739 |
| `perl-parse` CLI binary | #5736, #5737 |
| URI module UTF-8 | #5732 |
| `position-tracking` UTF-8 clamp | #5733 |
| Code-actions pragma detection | #5734, #5735 |

Only the first row is a real same-layer duplicate cluster. Everything else is cross-layer and complementary. **Correct triage is file-path-based, not title-based.** See `memory/feedback_broad_scope_codex_stack_diversity.md`.

**Cheap triage command:** `gh pr diff <N> --name-only` — if two PRs with similar titles touch disjoint file sets, they're layer-diverse, not duplicates.

### 3. Master bit-rot cascade — tier-wiring exposes accumulated debt in waves

Widening CI scope (#5005 tier-wiring + `cargo check --workspace --all-targets`) reveals compile / format / test errors that the narrower `--lib`-only push CI had been hiding for weeks. The exposure happens in waves — each fire-fix unblocks the next layer of failure.

**Fire-fix sequence on PR #5501 (combined master unblock):**

1. `scope_and_symbol_tests.rs:1737` `${Foo::name}` invalid format string (intro #5090)
2. `mojolicious_navigation_tests.rs:417` stray duplicate close (intro #5288 merge artifact)
3. `xtask/lsp_stats.rs` incomplete refactor from #5303 (`last_run`, `run`, `load_last_run` undefined)
4. `xtask/lsp_stats.rs` fmt drift from same #5303
5. `hash_key_bareword_tests.rs` 22 type errors (`expected &Arc<Node>, found &Node` API drift)
6. `perl-regex/tests/comprehensive_unit_tests.rs:366` fmt drift
7. `perl-workspace-index` fmt drift (function signature > 100 chars)
8. `perl-semantic-analyzer` fmt drift (3 long signatures)
9. `perl-dap/platform/mod.rs` fmt drift
10. Fixture matrix missing `confidence_signals` on scenario 18 workflows
11. `ci_scope.rs` widener rule stale crate refs (`perl-lsp-definition` etc. collapsed)
12. `parser_tests.rs:163` rustfmt split-form cascade (post-merge of #5395)
13. `incremental_v2.rs` fmt drift (post-merge of #5467) — followup PR #5751
14. `incremental/mod.rs` fmt drift (post-merge of #5465) — followup PR #5780

Each stage was discovered only after the previous one landed. The lesson: **widening CI scope produces short-term noise that IS the point**; it is exposure of real accumulated debt. Don't relax the gate; land the cascade.

Prescribed follow-up issue #4507 — make `cargo check --workspace --all-targets` a push-gate, not just a merge-gate, so new drift is caught at commit time not cascade time.

### 4. Deep review catches real bugs — not a rubber stamp

Across ~120 PRs deep-reviewed this session, the following **real production bugs** were caught and fix-forwarded. All would have shipped otherwise:

| Area | Bug | PR | Kind |
|---|---|---|---|
| Parser incremental | `offset_to_position_rope` panic on mid-codepoint offset | #5733 | Crash |
| Parser incremental | Insertion invalidation zero-length range | #5466 | Silent correctness |
| Rename feature | `find_occurrences_in_text` used OLD name as `new_text` — every rename in comments/strings was a no-op | #5418 | Silent correctness |
| Rename feature | `normalize_rename_target` returned bare name without sigil; `$count → total` produced `total` not `$total` | #5717 | User-visible |
| Cancellation | Cache invalidation gap — re-registered request reported cancelled due to stale Arc | #5428 | Silent correctness |
| Tree-sitter | Missing `tree-sitter-language` dep; code compiled only because hidden transitive dep | #5489 | Build fragility |
| FindBin | `$Bin` word-boundary bug — `$BinDir`/`$BinPath`/`$RealBinConfig` all matched | #5392 | Silent correctness |
| BDD tests | Assertion scan for "unknown" against AST that never emits lowercase "unknown" | #5395 | Vacuous test |
| Workspace rename | `is_sub_declaration_line` didn't split on `;`; forward declarations misparsed | #5434 | Silent correctness |
| Workspace rename | Inverted range bypasses package-context check | #5434 | Silent correctness |
| Formatting | `trim_final_newlines` stripped only ONE trailing `\n` (LSP spec violation) | #5657, #5665 | Spec violation |
| Formatting | `BuiltInFormatter::format()` double-decrement of `indent_level` on closing delimiters | #5666 | User-visible |
| Critic | `is_stringy_eval_line` word-boundary bug — `eval_result`/`myeval` falsely matched | #5710 | False positive |
| Critic | `find_bareword_open_filehandles` off-by-one (last 4 bytes never tested) | #5712 | Missed detection |
| Critic | PL403 diagnostic reports every line as `line: 1` (hardcoded) | #5711 | User-facing misinformation |
| Critic | `extract_open_statements` CRLF offset drift | #5711 | Cross-platform silent bug |
| UTF-16 | `chunks_exact(2)` silently drops odd trailing byte (not panic — data corruption) | #5742, #5743 | Silent data loss |
| nvim docs | Guard uses `lspconfig.perl_lsp or lspconfig.perl_ls` — neither exists; guard silently skips for new users | #5442 | Silent no-op for new users |
| Zed docs | Wrong config schema (`command`/`args` vs required `binary`/`arguments`) | #5470 | Non-functional |
| Emacs docs | `perl-ts-mode` presented as standard; it's a 4-star third-party experimental | #5444 | Misleading user |
| LSP harness | `collect_notifications` slept 120ms without re-entering read loop; post-hover `publishDiagnostics` silently dropped → systematic test flakiness | #5399 | Test flake with incorrect cause |
| Code actions | `self.source[..end]` panic risk on mid-UTF8-boundary byte offset | #5702 | Crash |
| UTF-8 pragma | `has_utf8_pragma` matched `use utf8_custom`, comment lines, and was case-sensitive for encoding layer | #5734 / #5735 | Three false-positive classes |

Pattern: in almost every cluster, reviewer-deep finds AT LEAST ONE real bug. That's a **15-20× ROI** per reviewer-deep call vs. letting the PR merge on builder's own tests.

### 5. Concurrent worktree contamination — structural orchestration risk

When many agents share worktree slots, branch-switching races destroy work:

- `agent-a` checks out `pr-X`, makes edits
- `agent-b` simultaneously checks out `pr-Y` in the SAME worktree
- `agent-a`'s edits are replaced before commit

Observed 4+ times this session. Mitigations tried:
- Sequential per-PR work in one worktree (fine if one PR at a time)
- Fresh worktree per PR (blocked by nested-worktree hook #4456)
- Main-checkout operation (leaks to user's pwd; causes branch drift)
- GitHub Contents API push (works for small single-file edits; bypasses worktree entirely)

The Contents-API pattern saved multiple fire-fix commits this session when local git was corrupted or races hit. Useful pattern for narrow edits; not a replacement for real worktrees on bigger work.

### 6. Stale-base false-positive in diff auditing

A PR branched BEFORE recent merges, when diffed against CURRENT master, shows the merged changes as "deletions". Earlier diff-audit agent this session flagged 29 PRs with "identical 1,317-1,320 line deletions across all 29" as scope-drift contamination. In reality: each PR branched before the 16 merges of that morning; the "deletions" were the 16 merged PRs' changes viewed from the pre-merge perspective.

**Correct triage:** check the PR's **three-dot diff** (`master...HEAD`, not `master..HEAD`) or use `gh pr diff` which does this by default. Mass-closing on a "massive deletion" pattern is the bug, not the signal.

### 7. Git identity corruption — must check at session start

Both local AND global `user.name` were reset to `test/test@test.com` at some point this session. Several commits went up as `test <test@test.com>`. Forensics: some sandbox init or hook violating the "NEVER update git config" rule. Created skill `/fix-git-identity` that restores the correct identity from the canonical noreply.github.com email and clears local overrides. See `memory/feedback_git_config_test_identity_leak.md`.

### 8. Orchestration meta-patterns

**What worked:**
- 20-agent parallel waves when queue is deep and work is non-overlapping
- Sequential-in-one-worktree for related PRs (avoids branch-switch race)
- Explicit "review ALL N" in Haiku prompts to prevent "stopped after first bug" behavior
- Layered pipeline: haiku → deep-review → refactor-planner → green-tdd → diff-audit → ops
- Autonomous-loop wake-ups at 240-300s (stays in prompt-cache window)
- File-path triage before title-triage for dup detection

**What burned cycles:**
- Fanning out haiku-review agents too tightly scoped (1 PR each) — overhead > work
- Large deep-review agents covering 30+ PRs — became shallow
- Agents sharing worktrees with concurrent branch switches
- Assuming agent reports without verifying (e.g., fmt form correction got WRONG direction 3 times)
- Not capturing the git-identity issue earlier — several commits went up as `test`

## Architectural findings that deserve follow-up

1. **Goto-label resolution should live in DeclarationProvider**, not LSP-layer regex (see architecture-review of #5728/#5729/#5730/#5731). Single-point-of-resolution pattern — all other symbol types use this.
2. **LSP initialize precedence stack** has 4 levels: `workspaceFolders[0].uri` → `rootUri` → `rootPath` → `cwd`. PRs #5693 (first) + #5695 (last) are complementary; #5692 redundant.
3. **Mojolicious whitespace-in-route-name** (#5591) is a real fix but was scoped in a poisoned-example Codex task; generalize fixture to real Mojolicious app before merge.

## Unresolved at session close

- #5501 / #5724 / #5749 master-unblock stack — CI intermittently fails on format-check cascade. Current state: fmt form variations require iteration because Windows vs Linux rustfmt expectations differ subtly.
- #5593 Windows 8.3 short-path canonicalization — plan-review complete (`dunce` crate doesn't handle 8.3 expansion; need `GetLongPathNameW`); builder-ready.
- #5748 CPAN corpus ratchet CI failing 10 nights — plan-review complete; builder-ready.
- #5715 parser string-interpolation error recovery — scout + red-tdd complete; builder-ready.
- #5716 semantic-tokens vs document-highlight write-modifier inconsistency — scout complete; plan-review pending.
- #5722 LSP 3.17 completionItem capability (labelDetails/snippet/resolve) — scout complete; plan-review pending.

## Session artifacts added

- **Memory files:**
  - `feedback_codex_framework_hallucination.md`
  - `feedback_broad_scope_codex_stack_diversity.md`
  - `feedback_git_config_test_identity_leak.md`
- **Skills:** `.claude/commands/fix-git-identity.md`
- **Issues filed:** #5494, #5495, #5496, #5498, #5499, #5593, #5653, #5658, #5715, #5716, #5722, #5723 (walker StatementModifier skip), #5748 (CPAN ratchet), plus sprint-evidence comments on #4507
- **Session-level PRs:**
  - #5497 v0.13.0-rc1 punch list
  - #5501 master fire-fix stack (9 commits, eventually merged)
  - #5670 session-3+4 economics retrospective
  - #5724 / #5749 / #5751 / #5780 post-merge residual fmt + compile fixes
- **This retrospective** — itself an artifact

## The bottom line

Session produced **41+ merges, 21+ hallucination closes, ~25 real bugs fix-forwarded**, and surfaced a **systematic Codex failure mode** (framework hallucination) with a prescribed detection gate (MetaCPAN pre-check). The spray-and-filter orchestration economics continue to work — Codex generates breadth, Claude filters for truth and coherence. The dominant cost this session was the master-unblock fire-fix cascade, which would have continued to be invisible without tier-wiring's scope expansion.

_Forensic captured during the session for future-session substrate. Paired with `docs/forensics/2026-04-23-tier-wiring-reviewer-fix-forward-session.md`._
