# Verify the Instrument

## The thesis

Every reporting layer can lie. A CI badge, a coverage percentage, an agent "done" claim,
a test-count delta, a diff summary — each is an instrument reading, not ground truth.
When a result looks too good or too bad, suspect the instrument before acting on the
reading.

This is not generalized skepticism. It is a targeted recognition that in a pipeline
of stochastic stages reporting to each other, each layer translates the previous layer's
output and introduces its own failure modes — misconfigured scope, stale cache, wrong
target, wrong SHA, wrong profdata. The measuring instrument is often the bug.

---

## Incidents where the instrument lied

### CI "green" on the wrong target (#1282, #1453)

Coverage CI ran with `--lib` flags, measuring only library units. Integration tests were
excluded from the measurement. The percentage appeared healthy and the gate appeared green.
No individual check failed — the check was answering the right question about the wrong
scope.

Fix (#1453): the coverage invocation was corrected to include integration tests. The
displayed percentage changed because the scope changed, not because coverage changed.
The instrument had been healthy-looking while blind to a whole category of execution.

**Lesson**: a green coverage gate answers "is this percentage above threshold?" not "is
the code covered?" Verify the scope the instrument is measuring before trusting its verdict.

### A master break masked by a too-narrow gate (#651, #1458)

A duplicate function was introduced. The CI gate active at merge time was scoped narrowly
enough that the duplication did not trigger a compile error in that gate's target set.
Main broke. Other branches and their CI runs then failed for reasons traceable to the
broken master, not their own changes.

Fix (#1458): the gate scope was widened to catch the duplication. The underlying failure
was present in the diff; the instrument simply was not looking at the right surface.

**Lesson**: a gate that passes is evidence the gate's scope is clean, not that the PR is
clean. When the gate scope is narrow, the margin between "gate passes" and "master breaks"
is exactly the untested surface.

### Coverage measuring the wrong profdata

A coverage transformation script read profdata from a prior run rather than the current
one. The line counts were stale. The percentage was plausible — close to the previous
reading — so no one flagged it. The transformation was correct code operating on the wrong
input file.

**Lesson**: correctness of the transformation and correctness of the input are independent.
An instrument can execute flawlessly on stale evidence.

### Agent "done" as a reporting layer

A stochastic stage (an LLM agent) reports success. That report is itself an instrument
reading. The agent may have:

- run tests against a stale branch, not the current HEAD;
- summarized a diff it did not actually re-read after its own edits;
- confirmed a file count that was correct before it created additional files;
- reported "CI green" from a label that was set before a new push landed.

None of these are hallucinations in the model-failure sense — they are scope mismatches,
stale reads, and summarization errors. They are the same failure modes that a human
developer makes when reviewing a diff from memory rather than from the current checkout.
The agent summary is one more instrument reading; it requires the same ground-truth check
any other reading does.

---

## The recognition heuristic

> When results look too good or too bad, suspect the instrument first.

Concretely:

- **Too good**: a coverage jump after a small change; a test count higher than the number
  of files touched; CI passes on a PR that adds a banned pattern; an agent reports "all
  tests pass" on a branch that was not rebased after a known-breaking merge.

- **Too bad**: a test failure on a line that was not changed; a coverage drop on code
  that was not touched; CI red on the same SHA that was green an hour ago.

Both directions indicate an instrument reading that diverges from expected ground truth.
The divergence is a signal to check the instrument before acting.

---

## Cheap counter-moves

These checks cost a few seconds and catch the most common instrument failures:

**Check the HEAD SHA.** Before accepting any CI result as current, verify the result's
SHA matches the current branch HEAD. A result on a prior commit is a reading on a prior
instrument state.

```bash
# Verify CI result is for current HEAD, not a stale push
git rev-parse HEAD
gh pr view --json headRefOid --jq '.headRefOid'
# If they differ, the CI reading is stale.
```

**Assert the changed value via a direct accessor.** If a function was changed, write a
test that calls it directly and asserts the new behavior — not a test that calls a caller
that calls the function. Indirect coverage creates a gap between "covered" and "the line
was executed with the case that matters."

```rust
// Weak: tests function_a which happens to call changed_function
// Strong: tests changed_function directly on the new input case
assert_eq!(changed_function(new_case), expected_output);
```

**Diff the agent claim against ground truth.** If an agent reports "modified 3 files",
run `git diff --stat`. If the count differs, the agent summarized from memory. The
ground truth is the diff, not the summary.

**Check gate scope before trusting a gate pass.** When a required check passes, read the
check's invocation to confirm it covers the surface the PR touches. A gate that runs
`--lib` does not cover integration test paths. A gate that runs one crate does not cover
workspace-level cascades.

---

## Repair, not rationalization

When an instrument is proven wrong, fix the instrument. Rationalization — "the CI failure
is noise, merge anyway" — is a valid one-time exception only when all of the following
hold:

1. Human or maintainer authorization for the exception.
2. Proof the failure is instrument noise, not behavior noise (the specific failure
   mechanism is identified and understood).
3. The affected behavior has been independently tested and reviewed through a path that
   does not depend on the broken instrument.
4. A follow-up issue or fix exists for the instrument itself.
5. Release notes do not cite the broken instrument's green reading as evidence of
   correctness.

The goal is fewer exceptions over time, not better exception-rationale prose. Each
instrument fix (#1453 scope correction, #1458 gate widening) restores the property that
RED MEANS STOP, GREEN MEANS PROCEED for that instrument. That property is worth more than
the convenience of merging past one red check today.

A repaired instrument makes every future red result meaningful again. An unrepaired
instrument gradually teaches the team to discount all red results — which is how
catastrophic merges happen.

---

## Position in the pipeline

This tactic sits under the broader posture of adversarial-by-default verification — treat
every artifact as evidence with a reliability profile. "Verify the instrument" is the
application of that posture to reporting layers specifically.

It is also a tactic under the slow stochastic compiler model: a pipeline of LLM stages
compiling vague intent into PRs through stochastic stages. Each stage is a translation
that can introduce its own instrument errors. The operator's job includes distinguishing
"the code is wrong" from "the instrument measuring the code is wrong." These require
different interventions: a wrong instrument requires scope repair, not a new builder pass.

Relation to the shift-left ladder: instrument verification belongs at every rung of the
ladder, not just the top. A spec acceptance criterion can itself be an instrument check —
"the adversarial test must invoke the changed function directly, not through a caller."
That criterion prevents the "covered but not exercised on the new case" instrument failure
from reaching CI.

---

## Summary

| Instrument | Common failure mode | Ground-truth check |
|---|---|---|
| CI badge / status | Stale SHA, wrong scope, advisory not required | Match SHA to HEAD; read gate invocation |
| Coverage % | Wrong profdata, `--lib` excludes integration | Read gate flags; assert direct accessor |
| Agent "done" claim | Summary from memory, stale branch read | `git diff --stat`; re-read HEAD |
| Test count | Counted fixtures or snapshots, not behavior tests | `cargo test -- --list` |
| Label / sign-off | Set before most recent push | Compare label timestamp to last push |

The heuristic is cheap. The instrument failure is expensive. Check the instrument before
acting on the reading.

---

## Relation to other patterns

- **Hazard-class invariants** (`hazard-class-invariants.md`) — Class 6 (Coverage /
  Measurement Integrity) is the adversarial test pattern for the most common instrument
  failure: a transformation that silently drops production lines. Add it as an acceptance
  criterion on any change to a coverage pipeline.
- **Shift-left ladder** (`shift-left-ladder.md`) — instrument failures are caught latest
  at the CI rung; direct-accessor assertions and gate-scope checks climb the rung to spec
  acceptance criteria, where they cost nothing per-PR once written.
- **Orchestrator substrate model** (`orchestrator-substrate-model.md`) — the substrate
  includes CI timing, gate scope, and required-check definitions. An operator who has not
  verified the substrate's instrument configuration cannot trust the fleet's green signals.
- **Human corrects substrate** (`human-corrects-substrate.md`) — when an instrument is
  broken, correcting it is a substrate correction, not a builder task. The operator
  identifies the instrument failure; the fix is encoded as a permanent scope or gate
  change, not a one-time merge exception.

