# The Slow Stochastic Compiler (with an Operator)

*Portable concept. Grounded in perl-lsp. See also: [verify-the-instrument](verify-the-instrument.md), [hazard-class-invariants](hazard-class-invariants.md), [shift-left-ladder](shift-left-ladder.md), [orchestrator-substrate-model](orchestrator-substrate-model.md), [human-corrects-substrate](human-corrects-substrate.md).*

---

## The Model

An agentic engineering pipeline takes **vague intent** as input — an issue, a bug report, a spec fragment — and
produces a **merged, tested PR** as output. Between those two endpoints sits a sequence of stochastic passes:
scout agents, research verifiers, spec planners, builders, reviewers, refactors, CI checks, diff auditors.
Each pass may be wrong. Each pass has a reliability profile. Each pass costs compute.

This is a **slow stochastic compiler**.

- **Source** = the issue/spec (the program the human wants written)
- **Passes** = agents in sequence (each transforms the artifact one level closer to shippable code)
- **Type-checker / verifier** = CI, coverage gates, xtask lint, snapshot tests
- **Codegen / emit** = the merge
- **The human** = the operator who tunes flags, repairs buggy passes, and decides what the compiler's model of
  reality should be

The analogy is not metaphorical. A conventional compiler's optimization passes can each introduce unsoundness;
you catch it with a verifier. An agent pipeline's passes each introduce hallucinations, scope drift, or
missed edge cases; you catch them with the next pass, with CI, and with review. The difference is that compiler
passes are deterministic and agent passes are stochastic — so the system needs more checkpoints, explicit
reliability profiles, and incident handling when a pass produces confident nonsense.

---

## Why "Operator", Not "Human-in-the-Loop"

"Human-in-the-loop" implies a gate on each action. That is neither accurate nor the goal. The human is not
reviewing every agent output before the next agent starts. The human is:

1. **Setting the compiler's model of reality** — branch policy, merge economics, risk appetite, what counts as
   done, which invariants are absolute, when to stop.
2. **Correcting the model when it drifts** — when agents hallucinate a file path, merge against a stale branch,
   or treat a measurement failure as a behavior failure, the human recalibrates the operating model, not just
   the individual artifact.
3. **Authorizing exceptions** — the rare case where a required check is a proven instrument failure, not a
   behavior failure, requires human authorization and documented evidence. This is not routine; it is a named
   exception class (a *verified treadmill-break exception*) that should happen less often as instruments
   improve.
4. **Deciding what ships** — sequencing, scope boundaries, go/no-go at release, what makes it into release
   notes as a clean claim vs. a known caveat.

The preferred names for this mode are **human-calibrated autonomous execution** and
**operator-guided stochastic compilation**. The human adjusts the compiler; the human does not write each
assembly instruction. This is ordinary engineering management applied to a situation where the implementation
units are cheap enough that the scarce work moves up-stack: choosing invariants, shaping specs, assigning
proof obligations, verifying results, correcting the operating model.

---

## Compiler-Analogy Mapping

| Compiler concept | Agentic pipeline equivalent |
|---|---|
| Source language | Issue / spec / acceptance criteria |
| IR lowering passes | Scout → accuracy-scout → plan-reviewer → spec-planner |
| Optimization passes | Builder → green-tdd → reviewer → refactor-planner → green-refactor |
| Type checker / verifier | CI, xtask fmt/clippy, snapshot tests, coverage gates |
| Linker / codegen | Merge to master |
| Compiler flags | Branch policy, merge economics, gate skip criteria, risk appetite |
| Pass that introduces a bug | Agent that hallucinates an API, drifts scope, or misreads a snapshot |
| Verifier catch | CI red, green-ci agent detects stale SHA, diff-auditor flags artifact |
| Compiler engineer repairs a buggy pass | Human recalibrates operating model; files instrument-fix issue |
| Release binary | Shipped version with release notes and proof bundle |

One implication stands out: **a buggy pass that is not caught by the verifier is a verifier gap, not just a
pass failure**. When an agent consistently hallucinates a class of error (e.g., treating test-count metrics
as behavior coverage, or treating a Codecov miscounting as a real coverage drop), the fix is to improve the
catching layer — add a scout pattern, add a CI check, add a spec hazard row — not just to re-run the pass and
hope. See [verify-the-instrument](verify-the-instrument.md) and PR #1453 (Codecov integration-counting fix),
PR #1425 (verify-the-instrument doctrine), PR #1458 (master-break caught by instrument gap).

---

## The Economics

Passes cost tokens and compute. Verifiers cost CI wall-clock and runner dollars. Both are real costs that
belong in the same budget.

The design question is: **where is the cheapest sufficiently reliable place to catch a given class of defect?**

The answer is a ladder:

```
Rust type system                →  cheapest, catches ID-space collisions at compile time
cargo check --all-targets       →  catches dangling imports, missing trait impls
clippy class lint               →  catches structural patterns (unwrap, Clone on Copy)
focused unit test               →  catches local logic errors, parser node kinds
spec hazard row                 →  catches known classes before the builder starts
haiku scout pass                →  catches scope drift, obvious misreads, file-path errors
deep review pass                →  catches logic errors, edge cases, missed contracts
full CI suite                   →  catches integration failures, cross-crate breaks
release smoke                   →  catches packaging, binary startup, end-to-end regression
```

The goal is not to always climb higher. The goal is to **catch recurring expensive classes at the cheapest
sufficiently reliable layer**. A class that appears once in a one-off PR does not justify a new xtask
validator. A class that appears in three consecutive PRs from different builders — e.g., NodeKind variant
misclassification (PRs #1457/#1459) — justifies promoting a spec hazard row and, if it recurs, a focused
clippy lint or newtype wrapper.

More CI lowers agent cost by catching defects earlier. But more CI adds its own costs: queue starvation, merge
churn, false positives, maintenance, instrument complexity. Unverified CI is the worst outcome: it adds wall-
clock cost without adding signal. The merge gate requires **verified green on the current HEAD SHA** — not a
stale green label from a previous push — precisely because a stale label is instrument noise.

Agent-cost and CI-cost are the same design problem viewed from different sides. Both are operability budget.
Both are maintainability economics. **Designing for cheap traversal by future agents** (greppable contracts,
precise file ownership, durable learning entries, indexed concept docs) reduces total pipeline cost just as
building for the team that maintains it reduces human maintenance cost.

---

## Agentic Engineering Makes Hidden Economics Visible

This is the thesis.

Conventional engineering has the same economics. The difference is that most of the costs are hidden:

- **Token cost** ≈ the hidden cost of reading a codebase with poor discoverability, duplicated context, or no
  concept index
- **Stochastic pass failures** ≈ the hidden cost of onboarding a developer to a poorly specified area, watching
  them re-discover the same edge cases the previous developer hit
- **Instrument failures that look like behavior failures** ≈ the hidden cost of a dashboard nobody trusts, so
  everyone waits for a second confirming signal before acting
- **Operator recalibration** ≈ the hidden cost of technical leadership: setting policy, correcting
  organizational models, deciding what ships

When a stochastic pipeline makes these costs explicit — token counts, pass failure rates, CI false-positive
rates, exception counts, learning entries per incident — the system becomes auditable. Red means stop. Green
means proceed. Exceptions become evidence-backed incidents with documented authorization, root-cause, instrument
fix, and follow-up issue. The implicit discipline of good conventional engineering practice becomes an
explicit, measurable operating model.

The goal is not a novel paradigm. The goal is **conventional engineering discipline adapted to stochastic
workers, stochastic evidence, moving branches, and uneven tool reliability** — so that the signal remains
trustworthy, the meter remains accurate, and the human operator calibrates the compiler's model of reality
rather than manually implementing each fix.

---

## Implications: What Hangs Off This Model

**[verify-the-instrument](verify-the-instrument.md)** — When CI is red, the first question is whether the
instrument is measuring the right thing. A stochastic compiler needs trustworthy meters. Fixing instrument
failures is not bureaucratic; it is restoring the signal that makes GREEN meaningful.

**[hazard-class-invariants](hazard-class-invariants.md)** — The spec hazard row is a cheap catch layer inserted
between pass-N and pass-N+1. The compiler-analogy maps it to a mid-pass invariant check: if a known class
recurs, promote the catch layer rather than relying on reviewer judgment each time.

**[shift-left-ladder](shift-left-ladder.md)** — Cost-tiered prevention: decide for each defect class which rung
of the ladder is cheapest and reliable enough. Do not over-build machinery for one-off risks; do not under-
prevent recurring classes.

**[orchestrator-substrate-model](orchestrator-substrate-model.md)** — The orchestrator routes but does not
execute. In compiler terms: the scheduler assigns passes to workers; the workers do not re-architect the IR.
The orchestrator's job is sequencing, gate checks, and routing decisions — one per pass, not loops.

**[human-corrects-substrate](human-corrects-substrate.md)** — When the compiler's model of reality is wrong
(stale branch assumptions, miscounted metrics, conflated claim and evidence), no individual pass fixes it. The
operator corrects the substrate — the operating model, the policy, the instrument — and the passes re-run
against corrected ground truth.
