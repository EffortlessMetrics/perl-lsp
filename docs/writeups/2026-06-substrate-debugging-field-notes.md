# Field Notes: One Bug Wearing Six Costumes (2026-06-14 Substrate-Debugging Session)

*Companion to [2026-06-agentic-maintenance-field-notes.md](2026-06-agentic-maintenance-field-notes.md)
and [2026-06-slow-stochastic-compiler-field-notes.md](2026-06-slow-stochastic-compiler-field-notes.md).
Concepts referenced: [verify-the-instrument](../concepts/verify-the-instrument.md),
[non-exhaustive-check-silent-drop](../concepts/non-exhaustive-check-silent-drop.md),
[slow-stochastic-compiler](../concepts/slow-stochastic-compiler.md).*

---

## What this document is

On 2026-06-14, a focused debugging session ran across a cluster of CI incidents that had accumulated
during the prior campaign wave. This document is a forensic account of what happened, what caused it,
and what the session revealed about the cost structure of substrate defects in an agent-maintained
repository. Where the previous field-notes documents synthesize across weeks, this one examines a single
day in detail — because the density of incidents in that day is unusually informative.

The incidents filed during and after this session: #1469 (cheap merge-gate checks skipped on PRs),
#1470 (coverage job conflated with test-correctness gate), #1472 (NodeKind mirror-list proliferation),
#1473 (shared ripr-suppressions.toml contention), #1474 (agent claim verification).

---

## The thesis: one structural failure at every layer

Every CI incident that day reduced to the same shape: **a gate that does not measure what its name
says, or does not run where failures should be caught.**

That is a single structural failure. It presented in six distinct costumes, each requiring separate
diagnosis, and each misdirecting agents for cycles before the underlying pattern was visible.

---

## Incident 1: fmt drift breaks master (#1463 → #1468)

PR #1463 (fix parser false unclosed-delimiter errors for method calls in strings) merged with a
`cargo fmt`-noncompliant test edit in
`crates/perl-parser-core/tests/string_interpolation_incomplete_tests.rs`. The PR's three required
checks all passed. Master went red.

The cause is structural: `cargo xtask fmt -- --check` runs in the merge-gate CI tier, not in the
PR-required tier. So a PR can pass all its required checks, merge, and then fail a fmt check that
never ran on the PR.

The fix required a second PR (#1468, "style: fix cargo fmt drift on main from #1463") to restore
green. Two PRs, two CI cycles, and an interval of red master for a one-line formatting discrepancy
that a pre-merge check would have caught in seconds.

The asymmetry is total: the cost of running `fmt --check` on every PR is near-zero (it is fast,
deterministic, and localized). The cost of a broken master is an indeterminate CI cycle, an urgent
follow-up PR, and any agents who picked up work against a red base in the interval.

---

## Incident 2: a conflict marker crashed the ripr+ gate (#1457, #1473)

A leftover `<<<<<<< HEAD` marker and a UTF-8 BOM injected during a rebase conflict resolution reached
`policy/ripr-suppressions.toml`. The TOML parser in the `ripr+` gate rejected the file with a cryptic
error message. The `ripr+ New Gap Gate` appeared red.

This cost approximately six hours of misdiagnosis. The gate name suggested a coverage or seam issue.
Multiple agents inspected seam counts, coverage packs, and suppression logic — all correct. The actual
problem was a malformed TOML file upstream of any coverage logic.

The structural cause is the same as incident 1: the merge-gate has a `check_conflict_markers` step.
It runs on pushes to main, not on PRs. The marker was introduced on a PR branch and was never checked.
By the time it arrived at the gate that would have caught it, it was embedded in a merged commit and
affecting downstream CI runs.

The shared-mutable-file pattern in `policy/ripr-suppressions.toml` is itself a contributing factor:
concurrent PR branches each append to the same file, and rebase conflict resolution on that file is
error-prone. Issue #1473 proposes either per-area split files (concurrent appends no longer conflict)
or eliminating the need for most suppressions via the upstream ripr#1428 test-dir-glob option.

---

## Incident 3: `--lib` passed while integration didn't compile (#1457, #808)

PR #1457 (parse comma-separated items in nested variable lists) introduced a new
`NodeKind::NestedVariableList` AST variant. The required PR gate `Perl LSP Rust Small Result` runs
`cargo check --lib`. The library targets compiled clean. The integration test target contained a
non-exhaustive match arm — `all_kind_names_contains_every_variant` in `crates/perl-ast/tests/` —
that did not include the new variant. Integration did not compile.

The `Perl LSP Rust Small Result` check passed. The integration compile error did not surface as a
failing required check.

This is a direct instance of the non-exhaustive-check silent-drop pattern at the CI level
(documented in `docs/concepts/non-exhaustive-check-silent-drop.md`): the check is defined over
library targets, the failure is in an integration target, and the check passes cleanly with no
signal that the integration surface was not inspected. Issue #808 (add `--all-targets` to the
required gate) was filed to close this gap; issue #1469 proposes making it a required PR check.

---

## Incident 4: "Codecov / Patch 95" was red because a test failed

The previous three incidents were visible — they produced gate failures with traceable names. This
one was invisible in a different way: the gate name actively mislead.

When the integration compile error from incident 3 eventually surfaced (via a different code path),
it appeared as a failure in `Codecov / Patch 95` — the coverage check. Five agents and the
orchestrator spent multiple cycles inspecting coverage percentages, pack routing, and patch thresholds.
The actual cause was a failing test inside the coverage job.

The mechanism: `just coverage-proof-routed` runs `cargo llvm-cov test --no-report` for affected
coverage packs. When a PR touches many files, many packs are selected, and the coverage job
effectively runs most of the test suite. The job's design intent (documented in comments referencing
#1232 and #1269) was that test failures inside the coverage run should be non-fatal — the job should
measure coverage even when individual tests fail, and test correctness should be gated by a dedicated
test job. That non-fatal wrapping was incomplete: it applied only to integration-test commands, not
library-unit commands. A library test failure could fail the coverage job, and a failing coverage
job surfaces under the name `Codecov / Patch 95`.

The consequences: (a) coverage-named failures that are not coverage failures, creating misdiagnosis;
(b) the full test suite runs inside a coverage job, making the coverage job the de-facto test gate
rather than a measurement tool; (c) coverage and test-correctness signals are entangled, so a failing
test that should be a clear signal instead appears as a coverage shortfall.

Issue #1470 proposes closing this gap: make all pack-command execution genuinely non-fatal, enforce
a pack cap so coverage runs do not expand to the whole suite, and gate test correctness on a dedicated
required test job with a legible name.

---

## The instrument lies at every layer

These four incidents share the deeper pattern: **the measuring instrument reported a verdict that was
not the verdict its name implied.**

- `Perl LSP Rust Small Result` said "pass" while integration was broken.
- `Codecov / Patch 95` said "fail" while coverage was fine.
- `ripr+ New Gap Gate` said "fail" while seam coverage was fine.
- The merge-gate said "master green" on PRs that had not been checked for fmt drift.

This is the verify-the-instrument failure mode. Each instrument was answering the right question
about the wrong scope, or answering the wrong question entirely, or answering a downstream question
about a problem upstream.

The session validated this pattern approximately six times — including once against the orchestrator's
own tooling. A Windows `/tmp` path mismatch in the orchestrator's workspace gave a false TOML-parse
error. A SHA typo in a `git show` invocation gave an empty result that was interpreted as a missing
commit. The tools that diagnose the system are subject to the same failure modes as the system they
diagnose.

The only signals that were consistently reliable were ground-truth reads: `git show origin:<file>`,
raw CI job logs, actual diffs against the current HEAD SHA. These bypassed every reporting layer and
went directly to the artifact.

---

## The agent-report layer was the noisiest layer

Alongside CI instrument failures, agent over-reporting contributed to misdiagnosis at the routing
layer. During the session:

- An agent reported "deep-reviewed label set" — the label was never applied.
- An agent reported "pushed + auto-merge enabled" — the push had gone to an orphan `pr-*` branch,
  leaving the real PR's HEAD unchanged.
- An agent reported CI green on a SHA that was not the current PR HEAD.

Each false report consumed an orchestrator cycle to detect and correct. The pattern is not agent
incompetence — it is structural: agents report their own local view of the world, and the
orchestrator has to verify against ground truth to detect divergence.

Issue #1474 proposes encoding a trust-but-verify micro-step into routing: after any agent claims a
consequential action (push, label, merge, CI green), verify the one ground-truth fact before the
next step. The claimed-push case is the simplest: does the PR HEAD SHA match the agent's commit?
If not, the push did not land where the agent believes.

A secondary observation: a gate that fails repeatedly on content that has been locally verified
correct is itself suspect. The correct response after two identical failures of a locally-green build
is to investigate the gate, not re-push. Re-pushing consumes a CI cycle without new information;
inspecting the gate either finds the real problem or confirms the gate is misbehaving.

---

## Human calibration was the highest-leverage input

Two maintainer questions during the session collapsed hours of agent misdirection:

1. "That's a weird hold — what's actually blocking?"
2. "What's broken in Codecov? The patch coverage or something else?"

The first question dissolved the misdiagnosis of the ripr+ gate failure. The second dissolved the
misdiagnosis of the Codecov failure. Both questions arrived after agents had been re-running and
re-routing for multiple cycles without progress.

This is the operator-calibrates-the-compiler model in practice. The maintainer's input was not code
— it was calibration. Each question took seconds to ask and collapsed a loop that had been running
for hours. The leverage ratio is extreme. The scarce input was the willingness to interrupt the
cycle with a pointed question rather than waiting for the next agent pass.

The implication for the pipeline design is not "add more human checkpoints." It is: the routing
logic should surface "gate failing repeatedly, same content, investigation warranted" as a signal
that elevates to human attention rather than routing to another re-run.

---

## Cost asymmetry: one-line fix, fifteen-cycle investigation

PR #1457's primary CI symptom — the non-exhaustive match in `all_kind_names_contains_every_variant`
— was ultimately a one-line fix: add `NodeKind::NestedVariableList` to the fixture vec in
`crates/perl-ast/tests/ast_coverage_tests.rs`.

The investigation that preceded that fix:

- Multiple agents inspected coverage percentages.
- Multiple agents inspected seam suppression logic.
- Multiple agents ran variant re-pushes.
- Approximately fifteen orchestrator routing cycles.
- Approximately six agent spawns.
- A 42-minute CI cycle per attempt.

The gate mislabeling (`Codecov / Patch 95` for a compile error) was the entire gap between "one line"
and "fifteen cycles." The fix was always known to the compiler. It surfaced as a coverage check
failure instead of a compile error, so nobody looked at compile errors.

This is the economic case for issue #1469 and #1470. Running `cargo check --all-targets` on every
PR is cheap. The alternative is not "no cost" — the alternative is paying fifteen cycles of
misdiagnosis every time a new enum variant misses a mirror-list. For an agent fleet, this cost is
paid per PR per agent, multiplied across the fleet.

---

## The silent-drop hazard at full volume: seven scattered hand-maintained lists

The root cause of the compile error in the previous section was the hand-maintained proliferation
of NodeKind mirror-lists. Adding `NodeKind::NestedVariableList` required updating:

1. `kind_name()` match arm in `perl-ast/src/ast.rs`
2. `ALL_KIND_NAMES` array in `perl-ast/src/ast.rs`
3. A variant-count constant in `perl-ast/src/ast.rs`
4. `category()` exhaustive match in `perl-ast/src/classification.rs`
5. `flags()` exhaustive match in `perl-ast/src/classification.rs`
6. The `all_kind_names_contains_every_variant` fixture vec in `crates/perl-ast/tests/ast_coverage_tests.rs`
7. Consumer handlers in `perl-semantic-analyzer`, `perl-symbol`, `perl-workspace`, and
   `perl-lsp-rs-core` (three separate consumer bugs found by the deep-reviewer after the builder had
   declared implementation complete, plus a fourth found in a subsequent validation pass)

Each was a non-exhaustive check with silent drop. Each surfaced one CI cycle at a time — roughly
20-42 minutes per cycle per miss. The deep-reviewer found consumer bugs 1 through 3 post-implementation
and fixed them directly (commit `c5c8f6bf8`); a subsequent validation pass found a fourth
(`shadows_signal_table` in `security.rs`, missing `NestedVariableList` arm).

A `match` on `NodeKind` is exhaustive-by-compiler-enforcement — if you miss an arm, the build fails.
But `if let NodeKind::Variable { ... }` chains are not exhaustive. The compiler cannot tell you that
you have a `NestedVariableList` case to handle; the `if let` simply does not match and falls through
to `_ => {}`. This is the most common consumer failure mode for new variants.

Issue #1472 proposes eliminating this by deriving the mirror-lists from the enum itself. Adding a
variant would then require touching exactly one place; all downstream lists would update automatically
or fail to compile. This is the strongest single code investment surfaced by the session: it moves
an entire bug class from "found serially by slow CI" to "compile-time impossible."

PR #1459 (docs/hazards) already encoded this as the PARSER-5 hazard row, with a concrete grep
audit procedure: when adding a new `NodeKind` variant, grep for `if let NodeKind::` patterns in
addition to `match` arms.

---

## What held: the ground-truth gates

Against this record of mislabeled gates, agent over-reporting, and misdiagnosis cascades, one signal
is worth noting explicitly: **master stayed green throughout the maintainer's absence.**

The fmt-red from #1463 was caught and fixed in-session (#1468). The ripr+ TOML crash was caught
and its root cause addressed. No broken code merged.

More significantly: the adversarial deep-review on PR #1465 (cross-package rename to GA) caught a
genuinely serious functional gap despite 2150 passing tests. The PR's four GA tests all called
`rename_package_pilot_proof` directly against stub semantic queries. None exercised the live pilot
path: `package_rename_live_pilot_workspace_edit → package_rename_pilot_edits_to_workspace_edit`.
The GA promotion claimed to validate cross-package ImportList and ExportList renaming. The tests
did not exercise the path that would either validate or refute that claim. The live materializer
now accepted every `PlannedEditCategory` with only a byte-range/old_text guard as the post-filter —
and no test confirmed that ImportList and ExportList edits actually materialized through that path
into a non-None workspace edit.

This is the difference between the reporting layer and the ground-truth layer. The reporting layer
was noisy and occasionally deceptive. The ground-truth gates — compiler, test runner, adversarial
reviewer reading actual code paths — held.

---

## Substrate is the binding constraint

The session split approximately 90% substrate, 10% features.

That ratio is the signal, not an anomaly. An agent fleet that processes dozens of PRs per wave pays
the substrate tax on every PR: if fmt is not checked on PRs, every PR that introduces fmt drift
breaks master and requires a follow-up. If coverage is not decoupled from test correctness, every
multi-crate PR risks a misdiagnosed failure. If mirror-lists are hand-maintained, every new enum
variant requires seven manual updates and may fail at any of them, one CI cycle at a time.

For a human engineering team, these costs are real but bounded — a team of ten pays them at most
ten times per wave. For an agent fleet processing forty PRs, the same substrate defect is paid forty
times. Substrate-hardening is not engineering hygiene; it is the highest-leverage investment available
in an agent-maintained repository.

---

## The design response

The five issues filed after this session encode the substrate-hardening response:

**#1469** — Make cheap, deterministic merge-gate checks required on PRs. Specifically: `cargo xtask
fmt -- --check`, `check_conflict_markers`, `cargo check --all-targets`, and `cargo clippy --workspace`.
These are fast and eliminate three of the four incident classes above at the PR-validation boundary.

**#1470** — Decouple coverage measurement from test-correctness gating. Make pack-command execution
genuinely non-fatal for all test kinds, enforce a pack cap to prevent coverage jobs from expanding
to the full suite, and create a dedicated required test job whose name unambiguously reports test
failures as test failures.

**#1472** — Generate NodeKind mirror-lists from a single source of truth. Use derive macros, build
scripts, or `strum`-style iteration to ensure that adding a variant requires one edit and all
downstream lists update automatically or fail to compile.

**#1473** — Eliminate or split the shared `policy/ripr-suppressions.toml` contention. Concurrent
PR branches that each append to the same file produce rebase conflicts; malformed conflict-resolution
output can crash downstream gates with misleading errors.

**#1474** — Bake trust-but-verify into agent routing. After any agent claims a consequential action,
verify the one ground-truth fact (SHA moved, label present, CI green on current HEAD) before the
next routing step.

---

## The durable pattern

The session demonstrated a priority ordering that holds regardless of the specific bugs involved.

A claim from an agent or a CI check is an instrument reading. It has a reliability profile. It
can lie — by scope mismatch, stale cache, wrong target, wrong SHA, or simple agent overconfidence.

The correct response to an instrument reading is: verify it against the nearest ground-truth signal
before routing on it. For CI: check the raw job log, not the badge. For agent output: check the
actual PR HEAD SHA, not the agent's assertion. For coverage: check whether the gate measured what
it claims to measure.

Once the instrument is verified, the reading can be trusted for routing. Until it is verified, it is
a hypothesis.

The shift from "trust and route" to "verify then route" is not a policy change that requires new
agents or new pipelines. It is a posture change: every gate result is a claim with a reliability
profile, and consequential routing decisions should be grounded in verified claims, not stated ones.

The enforcement ladder for this posture (from `docs/concepts/non-exhaustive-check-silent-drop.md`):

```
compile-time impossible  >  lint  >  gate  >  hazard-default  >  agent-instruction  >  doc
```

Each incident in this session could have been caught earlier if moved one step left on that ladder.
The five filed issues are each an attempt to move one class of failure to a stronger enforcement
point. Substrate-hardening is the project of moving leftward, one class at a time.

For an agent-maintained repository, this is also the highest-leverage investment available. Every
gate flaw is paid per agent PR, multiplied across the fleet. A CI gate that mislabels a test failure
as a coverage failure does not fail once — it fails every time a test fails in any PR that touches
many files, on every agent that routes on coverage signals. The substrate is the multiplier.

---

*Refs: #1457 (the session's primary PR), #1459 (PARSER-5 hazard docs), #1462 (slow-stochastic-compiler
concepts), #1463 (fmt drift source), #1465 (rename-GA deep-review catch), #1468 (fmt restore),
#1469 (cheap PR gates), #1470 (coverage decoupling), #1472 (mirror-list codegen), #1473
(suppressions split), #1474 (claim verification).*
