# 2026-06 Convergence-to-Release Plan

> This is a human-owned narrative plan, not a generated status doc.
> Live metrics belong in [docs/project/status/index.md](../status/index.md).
> Issue/PR numbers here are stable references, not status claims — check the
> tracker for current state.

## North Star

A Perl language server users can leave on all day: quiet at startup, accurate
in diagnostics, safe in edits, aware of project structure, useful in
tests/imports/methods/docs, released from evidence rather than hope.

Umbrella issue: [#1209](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1209).

## Milestone Ladder

The four milestones in order, from [#1209 comment](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1209#issuecomment-4640123789):

| # | Name | Gate | User outcome |
|---|------|------|--------------|
| M1 | Trust floor | queues reconciled · startup warning policy fixed · main green w/ fresh receipts · install/package smoke clean | "I can install it and it does not immediately annoy or confuse me" |
| M2 | Daily repair loop | top diagnostics have safe fixes · applyEdit trustworthy · missing-imports gated+receipted · @INC consistent across consumers | "It does not just tell me what is wrong; it helps fix it safely" |
| M3 | Semantic help | inline quiet/useful · context-aware test assertions · project-aware receiver methods · parse-safe suggestions · editor-native perldoc | "It understands the Perl I am already writing" |
| M4 | Release confidence | sync proven · artifacts match intent · all channels verified · docs do not overclaim · post-publish smoke | "The version I install is the version the project claims it shipped" |

No milestone may be declared complete until its gate criteria have evidence links.
Live capacity for each milestone is tracked in [#1209](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1209).

---

## Layer 1 — Queue Convergence (~90% complete as of 2026-06-07)

### What happened

The source queue (EffortlessMetrics/perl-lsp) accumulated ~130 PRs across
parallel swarm lanes during the trust-hardening phase. Convergence moved that
work into the swarm repo through a structured sequence:

- Merge trains [#9913](https://github.com/EffortlessMetrics/perl-lsp/pull/9913)
  and [#9914](https://github.com/EffortlessMetrics/perl-lsp/pull/9914) resolved
  the bulk of the open-PR queue.
- Consolidation merges [#9917](https://github.com/EffortlessMetrics/perl-lsp/pull/9917)
  and [#9918](https://github.com/EffortlessMetrics/perl-lsp/pull/9918) assembled
  clusters too interdependent to land individually; #9918 required follow-on
  fixes [#9921](https://github.com/EffortlessMetrics/perl-lsp/pull/9921) and
  [#9922](https://github.com/EffortlessMetrics/perl-lsp/pull/9922) after a
  reverted guard surfaced.
- Lineage merge [#9912](https://github.com/EffortlessMetrics/perl-lsp/pull/9912)
  attached the swarm lineage through a87f766ab.
- The swarm-side merge queue went live (ruleset: ALLGREEN, build+merge 3).

### Remaining steps

1. **Queue drain tail** — residual constituents not yet closed with receipts.
2. **[#1206](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1206)** —
   URI file-path acceptance + request-shape guidance port.
3. **[#1198](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1198)** —
   mirror v0.16.0 release prep from source.
4. **Train [#1207](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1207)** —
   merge the 27-PR reviewed train.
5. **Adapt-wave** — 14 mechanical ports, plus
   [#991](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/991) and
   [#930](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/930) to
   plan-review, plus
   [#682](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/682)
   adversarial review.
6. **Bidirectional sync-backs** — confirmed swarm fixes mirrored back to the
   source repo.
7. **Tree-convergence proof** — a one-time verification that both repos agree
   at the merge base; this should then become a standing CI check.

---

## Layer 2 — M1 Trust Floor

### Punch-list

- [#1220](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1220) —
  DAP test_binary_permissions_unix wrong install.sh path (closed/landed).
- [#1221](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1221) —
  3 insta snapshot failures on main (closed/landed).
- [#1226](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1226) —
  test_lsp_e2e_with_restored_modules stale package ID (open).

### Proof substrate hardened

CX routing: large-diff gates no longer land on capacity-limited runners —
[#1208](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1208),
[#1228](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1228),
[#1229](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1229).

Codecov rerun-safe: verdict from local receipt, upload non-fatal —
[#1231](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1231)
(swarm), [#9923](https://github.com/EffortlessMetrics/perl-lsp/pull/9923)
(source).

Queue fix: merge_group base-ref strip —
[#1249](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1249).

### M1 exit criteria

- Fresh CI receipts on converged tree (no stale green).
- Install/package smoke passes on converged HEAD.
- gen-tap confirmed swarm-only (no agent tooling reaching production paths).

---

## Layer 3 — Product Closure (post-M1)

Eight lanes in sequence, from [#1209](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1209#issuecomment-4640123789).
Each lane produces builder-ready child issues through the standard verification
pipeline. Lanes may run in parallel when they do not share a consumer or proof
surface.

1. **Startup trust** — defaults are quiet hints; explicit user config is
   actionable. No warnings about assumptions the server invented.
2. **@INC consistency** — one effective-@INC service; all consumers routed
   through it; negative fixtures for `no lib`, unreachable, and root-wildcard
   leaks. Anchors: #813, #812, #811.
3. **Repair loop** — diagnostic → precise explanation → safe edit →
   parse-stable result. Every meaningful diagnostic has a safe fix, an explicit
   no-safe-fix reason, or a documented issue.
4. **Missing-import next action** — gated, receipt-backed. Anchor: #935.
5. **Perldoc virtual docs + symbol docs** — `perldoc://` scheme, editor-native
   buffers via workspace/textDocumentContent, deterministic fallback. Builds on
   #1188.
6. **LSP 3.18 applyEdit metadata** — one complete safe-edit flow end-to-end
   with honest send/fallback receipts. Anchor: #1183.
7. **Bug fossilization** — every bug → repro fixture + regression test/receipt
   + close-with-pointer. Classes: startup noise, module resolution, diagnostic
   false-positives, unsafe quick-fixes, lifecycle races, unsafe-zone inline,
   parser recovery, transport DoS, Windows quirks, packaging.
8. **Proof routing** — changed surface → right proof pack → exact receipt →
   release summary. Anchors: #1196, #1126.

---

## Release: 0.16.1

Target: ships at convergence + M1 (owner decision 2026-06-07).

### Pre-dispatch checklist

- Lineage attach for the RC (merge-base verified, receipts attached).
- Proof bundle assembled (install/package smoke, fresh CI receipts,
  gen-tap swarm-only confirmation).
- Explicit owner approval before any publish step.

### Post-release verification

- GitHub Releases page updated.
- Backfill v0.15.1 and v0.15.2 GitHub Releases (currently missing).
- Channel verification: crates.io, VS Code Marketplace, Open VSX, Homebrew tap.

No channel may be declared verified from a label or doc claim alone — each
requires a live install/smoke receipt.

---

## Parallel Programs

### ub-review advisory integration

An 8-PR program hardening the unsafe-boundary review surface. Current position:
PR 1 merged ([#1234](https://github.com/EffortlessMetrics/perl-lsp-swarm/pull/1234) —
advisory workflow scaffold). Now in calibration phase before PR 2.

See [docs/ci/review-gates.md](../../ci/review-gates.md) for the sensor
definitions, upstream tool loop, and PR-3 conclusion-shape policy.

### CI-efficiency epic

[#1232](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1232) —
eliminate structural waste measured during the 2026-06-06 convergence.

- Merge queue proven (ALLGREEN, build+merge 3).
- Remaining: size-aware GitHub overflow per owner runner policy;
  advisory-neutral conclusions; source-repo queue flip.

Feature-gated tests need parallel `--lib` coverage: the gate measures the
default feature pack. A 28.57% coverage delta was observed when gated tests
were not included (context: [#1217](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/1217)).

---

## Standing Rules (from #1209)

- Deterministic local behavior first. AI assistance stays out until
  deterministic behavior is already boring.
- No default-on next-edit without receipts.
- No release until convergence + package/product smoke clean.
- No close without canonical reachability proof.
- No user-facing claim without test/receipt/smoke.

Operating loop per gap: reproduce → add regression → fix narrowly → validate →
merge → clean up → next.
