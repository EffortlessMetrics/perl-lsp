---
title: "Substrate tax and red-is-a-smell: two recalibrated operating principles from the 2026-06 campaign"
date: 2026-06
tags: [substrate, ci, merge, economics, anti-pattern, incident]
related_prs: ["#651", "#1282", "#1453", "#1458"]
search_terms:
  - substrate-tax
  - red-is-a-smell
  - verified-treadmill-break-exception
  - slow-stochastic-compiler
  - cheapest-sufficiently-reliable-layer
  - merge-past-red
  - instrument-repair
  - ci-cost
  - agent-cost
  - substrate-fix-amortization
  - coverage-scope-gap
  - gate-scope-narrow
---

## Lesson 1: Substrate tax

A substrate defect — a CI gate with wrong scope, a coverage invocation that excludes integration
tests, a required-check list that includes stale entries — is not a one-time cost. It is a **tax
on every subsequent PR** until it is repaired.

### Concrete incidents

**#1282 / #1453: Codecov `--lib` scope gap**
Coverage CI invoked with `--lib` only, silently excluding integration test lines. Every PR for several
months showed a coverage percentage that was technically correct for the library surface and
systematically wrong for the integration surface. The gate passed; the measurement was narrow. When
the scope was corrected (#1453), the percentage shifted — not because coverage changed, but because
the instrument's scope changed. The substrate tax was paid on every PR between the gap's introduction
and its repair.

**#651 / #1458: Narrow gate scope allowed duplicate function through**
The CI gate in use at merge time for #651 did not include the target that would have caught the
duplicate function introduction. Main broke. Every branch open at that time inherited a broken base.
CI runs for unrelated PRs failed for reasons traceable to the master break, not their own changes.
The substrate tax was paid by every agent working during the break window.

**Merge-cancellation cascade**
Rapid merges triggered Codecov upload cancellation. Coverage reports were incomplete for the merged
PRs. Downstream PRs had stale coverage references. The cascade was a substrate issue (merge pacing
policy), not a code issue.

### The leverage arithmetic

A substrate fix costs roughly one PR of engineering time. It amortizes across all subsequent PRs
until the next substrate change. A gate scope that is wrong for 30 PRs costs 30x the per-PR
substrate-diagnosis overhead. The fix costs 1x.

The signal that a substrate tax is accumulating: **agents routing around a defect rather than through
the correct path**. When agents repeatedly skip a check because "it always fails for unrelated
reasons," or when PRs are consistently merged with an override on a specific check, the override
pattern is evidence of a substrate defect that has been accepted rather than repaired.

### Operating principle

Fix the substrate. File an issue for the instrument repair before accepting the first override.
The accepted override is the beginning of the tax accumulation; the instrument fix is the amortization.

See `docs/concepts/slow-stochastic-compiler.md` for the broader framing: CI-cost and agent-cost are
the same design problem viewed from different sides. A substrate defect adds to both.

---

## Lesson 2: Red is a smell, not a routine

In an agentic pipeline, CI red means stop. Green means proceed. This is not aspirational — it is the
operating contract that makes the pipeline trustworthy. When red is routinely overridden ("this check
always fails for unrelated reasons"), the signal degrades: future red results are discounted, and the
override becomes the default path rather than the exception path.

### The verified treadmill-break exception

There is exactly one category of valid CI override in this pipeline:

**VERIFIED TREADMILL-BREAK EXCEPTION** — a one-time, documented, authorized override when ALL of:

1. **Human or maintainer authorization** — explicitly granted for this specific override, not as standing
   policy.
2. **Instrument failure identified** — the specific mechanism by which this check is producing a false
   result is documented. "It's probably a flake" is not sufficient. "The check invokes coverage with
   `--lib` and the changed function is in an integration test; the check cannot pass until #1453 fixes
   the scope" is sufficient.
3. **Independent verification** — the affected behavior has been tested and reviewed through a path that
   does not depend on the broken instrument. The override is not a substitute for the check — it is a
   bypass of the instrument while the behavior is verified through an alternative path.
4. **Follow-up issue or fix exists** — the instrument repair is tracked. The override is a bridge, not
   a permanent bypass.
5. **Release notes do not cite the broken instrument's green reading** — the override is not evidence
   of correctness. Release claims must be grounded in non-overridden evidence.

All five conditions must hold. A missing condition means the override is not a verified treadmill-break
exception — it is an unverified bypass. Unverified bypasses accumulate as substrate tax.

### The anti-pattern: red as routine

When an override becomes routine — "just skip that check, it always fails" — the pipeline has accepted
a degraded operating mode. The cost is invisible until a genuine failure rides through on the override
path. The 2026-06 campaign produced two examples where overrides masked real defects that only became
visible after the branch that carried the defect was merged.

The correct response to a routinely-failing check is not to accept the override. It is to fix the
check. If the check cannot be fixed immediately, disable it explicitly (not via override) with a
documented reason and a tracking issue. A disabled check is transparent; an accepted override is not.

### Operating principle

Red is a smell. When CI is red, the first question is: **is the instrument correct?** If yes, stop
and fix the code. If the instrument is broken, repair the instrument — do not merge past the red.
The verified treadmill-break exception exists for the rare case where instrument repair takes longer
than the merge window requires, but it is an exception class with strict validity conditions, not a
standing policy.

Fewer exceptions over time, not better exception-rationale prose. Each instrument fix restores the
property that RED MEANS STOP for that instrument. That property is worth more than the convenience
of one merge today.

See `docs/concepts/verify-the-instrument.md` for the instrument-repair tactic and the five-condition
checklist in detail.

## Related PRs

| PR | Description |
|----|-------------|
| #1282 | Coverage gate used `--lib` scope; integration test lines excluded from measurement |
| #1453 | Codecov scope corrected to include integration tests; substrate tax lifted |
| #651 | Narrow CI gate allowed duplicate function through; master broke |
| #1458 | Gate scope widened to catch duplicate; master-break class prevented going forward |
