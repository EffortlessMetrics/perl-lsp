# Scoreboard doctrine

How perl-lsp builds **measurement** (scoreboards) — the spine from "looks done" to "verified working." Load this before building any scoreboard. Tracking: [#3056](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3056) (+ #3057 integrity, #3058 unified, #3059 dogfood-first, #2674 compiler-backing).

> This doc deliberately carries its own rebuttal. A clean plan that hides its weak points is the exact failure mode it warns against. Read the **Pitfalls** as load-bearing, not appendix.

## Why measurement is the spine

The PR/issue board is perl-lsp's de-facto scoreboard — and it's ~40-56% wrong (measured: harvests find that fraction of issues already done-but-unclosed). Every hour of board-grooming is the **tax on not having a trustworthy measurement**. The fix isn't better grooming; it's a scoreboard that makes the truth cheap to query and impossible to fake.

The same defect recurs at every layer: a faked test (`#3036` hardcoded a `source_backed` receipt), a vacuous CI test, a done-but-unclosed issue, a lagging board. All are **a green signal decoupled from the truth it claims to measure.** Product correctness, CI gates, and the scoreboard are one problem at three scales: *make green mean something.*

## The three principles

1. **The oracle is the hard part, not the mechanism.** A scoreboard is trivial machinery around "what's the correct answer?" Two oracles, in priority: **perl-differential** (run real `perl X.Y`, pinned + cached — its answers are deterministic per version) where the answer is executable; **curated gold fixtures** where it isn't.
2. **Tamper-evident + fail-closed.** A faked scoreboard is worse than none — it manufactures false confidence. Every value must be *derived*, never a hardcodeable sentinel; fail closed on missing evidence; carry a SHA-anchored receipt.
3. **It must be consumed.** A scoreboard nobody gates on is inventory. Wire each to a merge gate / provider-promotion / roadmap decision.

## The sequence (corrected — demand-pull, not supply-push)

0. **Dogfood / capability audit FIRST** (#3059). Run the LSP on real CPAN modules + existing gold by hand. Produce a scrappy "what works / what's broken / what's already done" list. ~80% of measurement value for ~2% of cost — and it tells you whether the rest of this program is even warranted.
1. **Cheapest existing-oracle scoreboard.** The CPAN parser-ratchet already works (cheap oracle: does it parse). Extend from there.
2. **perl-differential oracle — built on demand,** pulled into existence by a *specific* failing family, not pre-built speculatively.
3. **Conformance scoreboards** per family, each gated on its own oracle.
4. **Compiler-backing scoreboard** ([#2674], keystone): % of provider answers that are truth-backed vs heuristic. This is the instrument that makes the compiler bet falsifiable — until it exists, PIR-A is an unfalsifiable investment.
5. **Unified rollup** ([#3058]) LAST: a human-steering view assembling the local scorecards. Not the agents' optimization target.

## Pitfalls (the rebuttal — these are where the strategy goes wrong)

- **Don't build the oracle first.** It's the most expensive, rathole-prone piece. Demand-pull it from a failing cell. (The original #3056 plan made it the prerequisite — backwards.)
- **A dynamic language has NO static ground truth for the hard cases.** `perl`'s *runtime* answer ≠ the correct *static* LSP answer. Method resolution / `@ISA` / magic / `tie` / `overload` / string-eval are resolved at runtime ([#2224], [#2221]) — `perl` itself doesn't know until it runs. The conformance scoreboard's honest ceiling is **< 100%**; scoring dynamic cases against a runtime oracle is *itself a faked measurement*. **Partition statically-decidable (oracle applies) vs. fundamentally-dynamic (best-effort vs. curated intent)** up front.
- **A measurement program can substitute for shipping.** A scoreboard tells the score; it doesn't score points. The scorecard issues already existed and languished — a coherent umbrella over them may be the Nth artifact that also goes nowhere. **Timebox; treat any scoreboard work that doesn't immediately redirect a decision as suspect.**
- **Integrity is a discipline, not a system** ([#3057]). The sufficient version is one *counter-assertion* per scoreboard (a fixture that must yield the opposite value, so a hardcode fails). Don't build a sentinel-lint before there are scoreboards to protect.
- **Goodhart-with-agents.** AI agents game a metric *immediately and competently* (vacuous tests, hardcoded receipts). Assume the measured party optimizes the literal gate, not the goal — tamper-evidence is the default, not paranoia. A unified single-source-of-truth is also a single number to game; prefer local scorecards consumed locally.
- **Oracle/fixture rot.** A scoreboard whose data flow silently breaks reports stale/empty as green (a faked scoreboard by neglect — the existing UX-scorecard "fix data flow" issues). Every scoreboard needs an owner + a freshness check (fail closed if the last measurement is stale vs HEAD SHA).

## See also

- [`docs/forensics/2026-06-25-orchestration-at-throughput.md`](../forensics/2026-06-25-orchestration-at-throughput.md) — truthful-closure as the binding metric.
- [`docs/forensics/2026-06-25-closure-gap-the-recurring-defect.md`](../forensics/2026-06-25-closure-gap-the-recurring-defect.md) — component-proved ≠ system-proved.
- The CPAN corpus ratchet (`just cpan-corpus-*`) — the one scoreboard that works, because its oracle is cheap. Use it as the template.
