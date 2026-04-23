# What the Repo Still Needs — Post-2026-04-23 Snapshot

**As of:** 2026-04-23 end of session window 2
**Context:** After landing ~250 PRs, 7 master bit-rot fixes, tier-wiring live, UX scorecard base in, agent receipt in, lane yield metrics in. Queue drained from 175 → ~40 open.

This doc catalogs what's still outstanding before **v0.13.0rc1**, grouped by urgency and category. Companions: `docs/articles/SESSION_2026_04_23_RETROSPECTIVE.md`, `docs/forensics/2026-04-23-tier-wiring-reviewer-fix-forward-session.md`.

## Release-blocking (must land before v0.13.0rc1)

### 1. Drain the remaining 40 PRs through merge

Most are DIRTY (need rebase) or UNSTABLE (CI settling). The rebase builder dispatched covers #5305, #5303, #5288, #5280, #5268, #5256, #5208. Open question: which of these are **genuinely additive** vs **replacable by merged work**? Triage the rebase-failures list and close superseded work rather than forcing all through rebase.

### 2. Resolve #5321 (Windows path tests)

Four real Windows-path tests failing in `perl-module/tests/path_comprehensive_unit_tests.rs`:
- `combined_edge_cases::many_include_paths_some_nonexistent`
- `legacy_separator_resolution::legacy_separator_resolves_via_path_after_normalization`
- `inc_search_order::include_paths_searched_in_array_order`
- `module_name_to_path_mapping::single_segment_module_maps_to_flat_file`

These block merge-gate on every PR that touches module resolution. Bit-rot #7 surfaced by the Windows Guardrails matrix (#5223 + #5317 unblocking it). Not optional — ship with broken Windows path semantics would hurt the users most likely to try v0.13.0rc1.

### 3. Stage the v0.13.0rc1 release itself

No PR this session stages the actual release. What's needed:
- Version bump across workspace `Cargo.toml` files
- CHANGELOG entry covering the session's landings (bit-rot fixes, tier-wiring, scorecard, agent receipt, Moose/Moo framework support, Perl 5.8-5.40 matrix, etc.)
- VS Code marketplace metadata + new VSIX bump
- Open VSX publish plan
- Crates.io publish dry-run + allowlist audit (post-session allowlist likely drifted after new crate additions)
- Release notes emphasizing: "framework-realistic Perl IDE support (Moose/Moo first-class), Perl 5.8-5.40 compatibility matrix, agent-consumable CI receipts, UX scorecard foundation"

## High-impact near-term (next session or two)

### 4. Incremental parse fuzz actually landing

#5279 (fuzz random edit sequences) passed deep-review + merged earlier. **But** the coverage is still narrow — ASCII-only content, property-testing over naive concat comparison. The structural silent-corruption class of bug (like the #4999 coordinate-space bug that almost shipped) needs:
- Non-ASCII / UTF-8 / surrogate-pair edit sequences
- Checkpoint invariant preservation across the fuzz runs
- Token-layer divergence detection between incremental and full-reparse paths

#5245 (fuzz-ish property test) merged with a tautology assertion flagged for follow-up. Strengthen to actually discriminate.

### 5. UX scorecard: from fixture-backed to measured

Merged this session: #5154 (fixture-backed scorecard), #5301/#5303/#5310/#5311 (metrics cohort — lane yield, receipt enrichment, ranking relevance fields). **But** the receipt still emits placeholders for several rows. Actually wire:
- Hover correctness % (needs real measurement harness against gold fixtures from #5307 schema)
- Completion top-1 / top-5 % (needs completion-ranking gold fixtures)
- Definition exact-hit %
- Symbol correctness %
- Cross-file success %
- P50/P95 latency per request class

Issues **#5306** (canonical UX harness), **#5307** (gold-fixture schema), **#5308** (scorecard numbers + ratchet policy) filed as Codex-prompt-ready specs for the next wave.

### 6. Windows matrix proactive lanes

#5223 + #5317 landed the Windows Guardrails matrix. **But** the matrix currently fires on every Windows-relevant PR. For cost: scope-trigger via ci-scope's `platform_overrides` output (already emitted by the classifier) so Windows lanes fire only when Windows-specific files / path code / sandbox code / CRLF-sensitive code changes.

### 7. DAP reference-client conformance

#5254 (DAP conformance sweep) approved and close to merging. **But** coverage is protocol-level replay. Real "DAP works" needs:
- Integration against `vscode-mock-debug` or a reference client
- Conformance testing across the full DAP surface (~37 handlers), not just negative/rejection paths
- Session-level tests: attach → breakpoint set → continue → break → inspect → step → disconnect

## Medium-term structural

### 8. Two-phase merge-gate

Proposal in `docs/articles/TWO_PHASE_MERGE_GATE.md`. Full merge-gate on first approval caches baseline; subsequent pushes only run scoped delta. Expected: 290+ runner-minutes saved per cascade-update storm (observed this session). Not blocking v0.13.0rc1; post-release throughput optimization.

### 9. Cross-tool parity

UX harness exists (`perl-lsp-ux-tests`). But test is LSP-protocol-level. Real "editors experience it right" needs scripted runners for VS Code, Zed, Neovim, Helix — each opens a fixture, measures completion latency + correctness, diff-checks against expected. Without this, a completion change can regress in a specific editor and slip through.

### 10. Haiku-pool agent pattern

The Haiku batches this session validated the cost hypothesis (~0.1% session per outcome vs ~1% for Sonnet). But green-tdd agents hit a worktree-branch-switching constraint (see `feedback_green_tdd_worktree_constraint.md`). Next iteration: a **Haiku pool** agent type that takes a list of target PRs and uses `gh api -X PUT` contents-PUT to push test additions to each branch without local checkout. Would scale the green-tdd pattern to 20+ PRs in one agent.

### 11. Semver contract enforcement

Merged #5259 documents the semver contract for allowlisted crates. But nothing gates `git push` against it — a breaking change on `perl-parser`'s public API could slip through. Needs: `cargo semver-checks` integrated as a conditional CI lane fired when the ci-scope classifier flags `public_api` risk tag (already emitted).

### 12. Agent-consumable receipt in use

#5263 enriched the gate receipt with `agent_receipt` block. But no agent actually consumes it yet — pr-responder still reads log tails. Next iteration: pr-responder wires into `agent_receipt.failures[].repro` directly and the receipt becomes the input protocol instead of stdout parsing.

## Lower-priority / nice-to-have

### 13. Release-engineering substrate

`cargo xtask release-prep` that stages the version bump + changelog + marketplace metadata + publish-dry-run + allowlist audit as one command. Would make v0.14, v0.15 releases a button-push rather than a checklist.

### 14. Flake registry

Observed this session: UX Regression Gate flakes under batch CI load. Timeouts bumped (#5097) but the flake pattern itself deserves a registry: each flaky test gets a label, auto-quarantine after N consecutive fails, re-queue nightly. Filed as #4823 earlier; not touched this session.

### 15. Cost telemetry

Each PR's review cost is currently private to my session metrics. A per-PR cost accountability ledger ("this PR cost $0.09 in reviewer-deep + $0.01 in standards reviewer + $0.03 in green-tdd") would expose noise-PRs that aren't worth the review budget.

### 16. Live orchestrator dashboard

Currently I pull PR state via `gh` for every decision. A live dashboard ("3 PRs in review, 2 waiting for CI, 1 stuck on merge conflict, 1 needs your decision") would eliminate a large fraction of my "check state" tool calls — a measurable budget saving.

## Documentation completeness

### 17. AGENTS.yaml (structured protocol)

`CLAUDE.md` + `AGENTS.md` are prose-format. Codex reads them but doesn't parse them. A machine-readable `AGENTS.yaml` with explicit rules (e.g., `{ "pre_push_hook_windows": { "symptom": "file-lock race", "workaround": "gh api -X PUT contents" } }`) would let both Claude and Codex consume the same substrate programmatically.

### 18. Forensic template consistency (#5318)

Filed — session forensics should consistently use window-scoped multi-checkpoint tables. Template should live in `.claude/agents/wisdom.md` so every forensic inherits the format.

### 19. Release-history ratchet

The `.release-history` file currently tracked manually. Auto-update via xtask post-merge hook.

## The meta-observation

**The repo's biggest remaining gap is not "more tests" or "more features" — it's product-experience validation.** We test protocol correctness. We don't test "user opens Catalyst app in VS Code, is the first 5 minutes good?" The UX scorecard + harness + gold-fixture work (issues #5306/#5307/#5308 filed) is the bridge between the two, but it's still scaffolding. When a scorecard run says "87% hover correctness, P95 latency 340ms, 3 categories of false-positive completions" — and that number moves deliberately through a ratchet — **then** we can announce v0.13.0 with confidence.

Until then: tier-wiring, fix-forward, Haiku offloading, and 250-merge sessions keep the machinery humming, but the product-quality story is narrated, not measured.

---

_Related: `docs/project/ROADMAP.md`, `docs/project/PRE_ANNOUNCEMENT_CHECKLIST.md`, `docs/articles/SESSION_2026_04_23_RETROSPECTIVE.md`._
