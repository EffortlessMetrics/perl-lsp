# Cheap Passes Beat Expensive Passes

The foundational insight behind agent swarm development.

## The Traditional Model

One senior developer reviews a PR for 20 minutes. That's one lens, one perspective, one moment in time. They're checking for correctness, style, security, performance, scope, and test quality — all at once, in one pass. They catch 60-80% of issues. The rest ship.

This is expensive, inconsistent, and creates a bottleneck. Review quality depends on who reviews, when, and how tired they are. Important PRs wait in queue. Trivial PRs get rubber-stamped.

## The Swarm Model

Five specialized agent passes at 30 seconds each:
- **Standards checker**: banned constructs, formatting, commit conventions
- **Security reviewer**: input validation, path traversal, unsafe blocks
- **Scope checker**: does this PR do one thing? Files outside scope?
- **Performance reviewer**: allocations, hot paths, caching opportunities
- **Test quality reviewer**: assertions meaningful? Coverage real?

Total time: 2.5 minutes of compute. Total cost: $0.50-2.00. Coverage: broader than any single reviewer.

## Why This Works

Each agent review is shallow compared to a deep human review. But:

1. **Breadth > depth for most PRs.** Most PRs are 10-50 lines. They don't need 20 minutes of deep analysis. They need a quick check across 5 dimensions.

2. **Consistency is a superpower.** The security review agent checks every PR, every time, with the same rigor. Human reviewers have good days and bad days.

3. **Specialization compounds.** A security-focused review agent encodes every security pattern the team has ever learned. A human reviewer carries whatever they happen to remember.

4. **Non-blocking.** Five parallel agents reviewing in 30 seconds don't block the merge queue. One human reviewer taking 20 minutes does.

5. **Scale is free.** Adding a 6th review lens costs nothing. Adding a 6th human reviewer is a hiring decision.

## The Implication

You can afford to throw many cheap passes of improvement at every change. Not just review — but also:

- **Post-merge validation**: did this actually help? (30 seconds)
- **Mutation re-testing**: does the new test actually catch the bug? (60 seconds)
- **Docs check**: does the README still match? (10 seconds)
- **Dep check**: did this introduce an unused dependency? (5 seconds)

Each check is trivially cheap. Together, they create a quality bar that no human team could maintain manually.

## The Cost of NOT Doing This

A bug that slips through review costs hours to debug, days to fix, and a hotfix release. A stale README confuses every new contributor. An unused dependency is one more CVE surface.

The question isn't "can we afford all these cheap checks?" It's "can we afford NOT to run them?"
