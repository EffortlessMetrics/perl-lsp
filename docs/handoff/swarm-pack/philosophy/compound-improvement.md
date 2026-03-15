# Compound Improvement: The ~20% Allocation

Why dedicating a permanent slice of capacity to codebase health creates compound returns.

## The Default

In most teams, improvement work happens in bursts: "test week," "docs sprint," "tech debt day." These are well-intentioned but episodic. Between bursts, debt accumulates. The codebase degrades continuously and is improved in batches.

The swarm inverts this: **improvement is continuous, degradation is episodic.**

## The Allocation

~20% of swarm branches always go to background improvement:
- **improver-docs**: ADRs, changelog, friction log, README, roadmap
- **improver-tests**: mutant killing, flaky fixes, coverage, integration gaps
- **improver-devex**: error messages, tracing, tooling, onboarding
- **improver-infra**: unused deps, dead code, security, build performance

This isn't "nice to have" capacity that gets borrowed when core work is busy. The improver teammates run continuously, in parallel with builders and reviewers.

## What Compounds

### Test Quality
Cycle 1: Kill 3 mutation survivors. Test suite catches 3 more bug categories.
Cycle 10: 30 mutants killed. Test suite is meaningfully more trustworthy.
Cycle 50: 150 mutants killed. The test suite is a genuine safety net.

### Documentation
Cycle 1: Write 2 ADRs for architectural decisions.
Cycle 10: 20 ADRs. New contributors can understand most design choices.
Cycle 50: 100 ADRs. The project has comprehensive architectural documentation that evolved organically.

### Developer Experience
Cycle 1: Improve 3 error messages.
Cycle 10: 30 error messages improved. Debugging is noticeably faster.
Cycle 50: Most error paths produce helpful messages. New contributors don't get stuck on cryptic errors.

### Infrastructure
Cycle 1: Remove 2 unused dependencies.
Cycle 10: 20 fewer deps. Smaller attack surface, faster builds.
Cycle 50: Dependency tree is lean. Security audit is clean. Build is fast.

## Why ~20%?

- **Less than 20%**: improvement can't keep up with degradation from core work. Debt still accumulates, just slower.
- **20%**: improvement roughly matches degradation rate. Codebase quality stays level or improves slightly each cycle.
- **More than 20%**: core work slows noticeably. Only worth it during explicit "lock down quality" phases.

20% is the steady-state allocation. Use `/swarm improve` for 100% improvement capacity when you want to pay down accumulated debt.

## The Insight

No single improvement PR is impressive. Removing one unused dep, killing one mutant, writing one ADR — each is trivial.

But a codebase that has had 50 cycles of continuous improvement is in a fundamentally different shape than one that hasn't. The improvements compound because each one makes the next cycle faster, the next fix safer, the next agent more effective.

**The codebase gets better at getting better.**
