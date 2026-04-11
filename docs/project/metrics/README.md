# perl-lsp Metric Stack

How the project measures itself, and how to think about which scorecard a PR improves.

> **Audience**: contributors and external readers who want a mental model for how progress is measured on `perl-lsp`. This page is the contributor-facing summary. The full design capture lives in the tracking issues linked at the bottom.

## TL;DR

- `perl-lsp` has a lot of numbers — parser cleanliness, capability coverage, mutation score, completion latency — but they answer different questions and are easy to conflate.
- The fix is **one scorecard per subsystem**, each answering the same four questions: **coverage, correctness, real-user behavior, cost**.
- Each scorecard is a **machine-readable metric file** under `.ci/metrics/<subsystem>.json`, split into **floor metrics** (must not regress) and **improvement metrics** (tracked but not blocking).
- Floors move via a **ratchet** — a metric only raises its floor after it has been stable across N runs, so a lucky green run cannot lock in a number we cannot hold.
- Every open issue should be able to answer the question: **which scorecard does this improve?**

If you take away one sentence: **the project has a shape problem, not a scarcity problem** — we already have enough telemetry, we just need each number to belong to exactly one scorecard and each scorecard to answer the same shape of question.

## 1. Why layered scorecards

For a long time `perl-lsp` tracked a handful of headline numbers — parser clean-parse rate, `features.toml` capability coverage, mutation score on the hottest crates, LSP request latency for a few flagship requests. Each of those numbers was real, but they were being read as if they were comparable, and they are not:

- **Parser clean-parse rate** answers *"how much of real Perl can we ingest without erroring?"*
- **Capability coverage** answers *"how many LSP methods do we respond to at all?"*
- **Mutation score** answers *"how confident are our tests that the logic under them is actually exercised?"*
- **Request latency** answers *"what does it feel like to use the editor on a big project?"*

Reading those as a single "health score" hides two failure modes at once:

1. **Overselling** — a crate can have 95% mutation score and still ship diagnostics that misfire on a dozen common idioms, because mutation score does not know what a false positive is.
2. **Underselling** — a subsystem can be in much better shape than the headline number suggests, because the catalog driving the percentage is itself incomplete.

The underselling pattern is not hypothetical. In issue [#4107](https://github.com/perl-lsp/perl-lsp/issues/4107) we audited the DAP handler catalog in `features.toml` and found 14 implemented handlers that were simply missing from the catalog. Correcting the catalog raised the public capability count from 102 to 116 — a 14-point jump with **zero new code written**, purely by fixing the measurement. If the old headline had been the only scorecard, the DAP subsystem looked worse than it actually was.

The fix is to stop trying to collapse every subsystem into one number and instead maintain one scorecard per subsystem, each shaped the same way. A reader can then compare apples to apples (*"how is the diagnostics scorecard trending this month vs last month?"*) without accidentally comparing diagnostics precision to parser coverage.

Each scorecard answers the same four questions, in the same order:

| Question | What it measures | Example |
|----------|------------------|---------|
| **Coverage** | How much of the relevant input space do we even try to handle? | Parser: fraction of Perl constructs with at least one green test. DAP: fraction of DAP protocol methods wired to a handler. |
| **Correctness** | When we do try, how often are we right? | Diagnostics: precision and recall against a labelled corpus. Parser: clean-parse rate and node-kind correctness. |
| **Real-user behavior** | What does it look like in the editor, on a real project? | Editor: completion relevance on top-idiom files. Workspace: reindex latency on a 10k-file workspace. |
| **Cost** | What does it cost to run? | Per-request latency, per-crate memory peak, startup time, mutation testing wall clock. |

That shape is what makes the scorecards layerable. The parser scorecard does not blend into the diagnostics scorecard, but both answer "coverage / correctness / real-user / cost", so a contributor can read any scorecard without learning a new framework.

### Why "shape" is the load-bearing word

"Shape problem, not scarcity problem" is shorthand for: the raw numbers already exist, they just do not line up with each other. Consider what an answer to "how healthy is the parser?" used to look like before this framing:

- Clean parse rate is 98.7% — from the corpus sweep job.
- Mutation score is 71% — from the mutation job.
- p99 parse time is 8.5ms — from a benchmark run.
- Node-kind coverage is 82% — from a one-off audit script.
- Capability count is 58/58 — from `features.toml`.

Each of those numbers is real. Each of them is produced by a different job, at a different cadence, into a different output format. An agent or a human wanting a single parser answer has to join five unrelated data sources and know which of them is the "real" headline number. That join is where drift and conflation come from.

The shape fix is: collapse those five sources into **one** scorecard file for the parser, shaped like every other scorecard file, with an explicit floor-vs-improvement split. The numbers do not change. Only their arrangement does. That rearrangement is load-bearing, because it means an agent or a human asking "is the parser healthy?" reads exactly one file and gets exactly one answer.

### A worked contrast: two "health" numbers that mean different things

Consider two numbers that both used to get reported as "parser health":

1. **95% of corpus files parse cleanly.**
2. **95% of tested parser branches pass mutation.**

They look equivalent. They are not.

The first is a **coverage-adjacent correctness** metric: it says "of the Perl we have seen, 95% is handled." It can regress because we added new files to the corpus, or because we added support for a new construct that exposed a lexer bug. It moves when the *input* changes.

The second is an **engineering-health** metric: it says "95% of the lines we test are actually exercised by the tests." It can regress because someone added a test that does not assert anything, or because someone added a code path without adding a test for it. It moves when the *tests* change.

Under the old "one parser health number" framing, those two regressions looked identical — both went from 96% to 95%, so both looked mild. Under the layered framing, the first belongs on the **parser** scorecard under "correctness" and the second belongs on the **engineering-health** scorecard under "cost of change." A reviewer can then give each one the attention it needs without guessing which was which.

## 2. The 7 scorecards

There are seven subsystems, each tracked by its own scorecard. Each has a design tracking issue; when a scorecard ships, it will live in `.ci/metrics/<subsystem>.json` with a human-readable summary in `docs/project/metrics/<subsystem>.md`.

| # | Scorecard | Tracking issue | What it measures |
|---|-----------|----------------|------------------|
| 1 | **Parser** | [#4063](https://github.com/perl-lsp/perl-lsp/issues/4063) | Clean parse rate, error density, recovery salvage ratio, node-kind coverage. "Can we ingest real Perl without erroring, and when we can't, do we recover usefully?" |
| 2 | **Diagnostics** | [#4065](https://github.com/perl-lsp/perl-lsp/issues/4065) | Precision, recall, false-positive rate, top-idiom diagnostic suite against real corpora. "When we flag a problem, is it actually a problem?" |
| 3 | **Editor intelligence** | [#4066](https://github.com/perl-lsp/perl-lsp/issues/4066) | Completion relevance, hover correctness, goto-definition hit rate, rename success, document-symbol completeness. "Does the editor feel smart on real code?" |
| 4 | **Module resolution** | [#4067](https://github.com/perl-lsp/perl-lsp/issues/4067) | Conformance matrix across resolution modes (workspace, absolute, lexical `use lib`, `FindBin`) and consumer consistency across LSP providers. "Do all features agree on where a module came from?" |
| 5 | **Workspace / indexing** | [#4068](https://github.com/perl-lsp/perl-lsp/issues/4068) | Initial build time, reindex latency, stale-entry rate, multi-root correctness. "How fast does the index converge and how often is it wrong?" |
| 6 | **DAP** | [#4069](https://github.com/perl-lsp/perl-lsp/issues/4069) | Launch / attach / variables / evaluate / end-to-end scenario success, truncation fidelity. "Can we actually debug a Perl program end to end?" |
| 7 | **Engineering health** | [#4070](https://github.com/perl-lsp/perl-lsp/issues/4070) | Per-crate mutation score, per-subsystem latency and memory, flaky-test tracker, release gate receipts. "Is the code underneath the features in shape to keep moving?" |

All seven scorecards are governed by the umbrella design tracking issue [#4062](https://github.com/perl-lsp/perl-lsp/issues/4062).

Each scorecard is scoped deliberately. The engineering-health scorecard is **not** a dumping ground for "everything else" — it specifically holds the cross-subsystem reliability metrics (mutation, flaky, latency, memory). If a metric fits one of the other six scorecards, it belongs there.

### Which scorecard owns which crates?

To keep the mapping explicit, here are the primary crates that feed each scorecard. A crate can feed more than one scorecard — for example, `perl-parser-core` contributes parsing cost to the parser scorecard and mutation score to engineering health — but it will have **one primary** scorecard for feature work.

| Scorecard | Primary crates (non-exhaustive) |
|-----------|-------------------------------|
| Parser | `perl-parser`, `perl-parser-core`, `perl-lexer`, `perl-ast`, `perl-token`, `perl-quote`, `perl-regex`, `perl-heredoc` |
| Diagnostics | `perl-lsp-diagnostics` and `perl-semantic-analyzer` providers, plus the linting and anti-pattern crates |
| Editor intelligence | `perl-lsp-hover`, `perl-lsp-completion`, `perl-lsp-definition`, `perl-lsp-references`, `perl-lsp-rename`, `perl-lsp-symbols`, `perl-lsp-signature` |
| Module resolution | `perl-module-*` family, `perl-workspace-index` consumer wiring |
| Workspace / indexing | `perl-workspace-index`, `perl-workspace-*` family, `perl-lsp` startup glue |
| DAP | `perl-dap`, `perl-dap-*` family |
| Engineering health | Cross-cutting — every crate contributes, but the scorecard is owned centrally |

If you are not sure which scorecard your PR belongs on, start from the primary crate you are editing and cross-check with the four-question shape. A change in `perl-lsp-completion` is almost certainly editor intelligence; a change in `perl-parser-core` error recovery is almost certainly parser; a change in `perl-workspace-index` on a watcher path is almost certainly workspace / indexing.

## 3. Reference-model inspiration

The instrumentation patterns on each scorecard come from how other language servers expose their own telemetry. The full survey is in [#4099](https://github.com/perl-lsp/perl-lsp/issues/4099); the short version is:

- **rust-analyzer** — uses `analysis-stats` to print per-phase timing, memory, and item counts against any crate, and `RA_PROFILE` as an env-var-driven tracing gate. Influences how the parser and workspace scorecards frame per-phase cost.
- **pyright** — `--stats --verbose` emits a structured breakdown of bind / check / parse cost per file. Influences how the editor-intelligence scorecard separates parse cost from analysis cost.
- **gopls** — exposes startup time, memory, and telemetry channels out of the box, including structured event logging. Influences how the workspace scorecard frames cold-start vs warm-index cost.
- **clangd** — ships `$/memoryUsage` as an LSP extension that reports live memory consumption. Influences how the engineering-health scorecard reports per-crate memory rather than one aggregate number.

The common thread is that each of these projects exposes **machine-readable, per-phase** metrics, not just a wall-clock number. Our scorecards follow that pattern: a scorecard is a JSON file an agent can read, not a paragraph a human has to interpret.

A secondary theme across all four reference projects is that their metrics are **opt-in for users** but **always-on in CI**. The same split applies here: a contributor running `perllsp` against a private project should not have instrumentation imposed on them, but every CI run on `perl-lsp` itself should record the full scorecard set. This keeps the public product quiet while keeping the development loop honest.

See [#4099](https://github.com/perl-lsp/perl-lsp/issues/4099) for the full research, including the exact invocations and output formats for each tool.

## 4. The ratchet model

A scorecard only matters if its floor can be trusted. The ratchet model, designed in [#4105](https://github.com/perl-lsp/perl-lsp/issues/4105), has four layers that together prevent both silent regressions and accidentally locking in numbers we cannot hold.

### Layer summary

| Layer | Rule | Why |
|-------|------|-----|
| **1. Machine-readable file** | Each subsystem gets exactly one `.ci/metrics/<subsystem>.json` file. | A single, parseable source of truth per subsystem. No scraping markdown, no ambiguity about which number is canonical. |
| **2. Floor vs improvement split** | Every metric is classified as **floor** (must not regress, blocks merge) or **improvement** (tracked, reported, not blocking). | Keeps the blocking set small enough to actually defend, while still giving credit for trending-up metrics. |
| **3. Ratchet only on stable wins** | A floor only raises after the improvement has held steady across N consecutive runs (specified per scorecard). | One lucky run does not lock in a number we cannot hold on the next PR. |
| **4. Every issue ties to a scorecard** | Every open issue should name exactly one scorecard it improves, or explain why it is off-scorecard (infrastructure, docs, tooling). | Surfaces work that has no home — which is usually either out of scope or a sign we are missing a scorecard. |

### Minimum-version rule

When a scorecard is first stood up, it must meet a **minimum-version rule**: it must contain at least one floor metric per question category that actually moves in real life.

- **One floor correctness metric** — "when we try, are we right?"
- **One real-user behavior metric** — "does this feel OK on real code?"
- **One latency or cost metric** — "does this fit in a human-feeling budget?"

Coverage is tracked but typically not floored in the first version, because coverage floors are too easy to game by shrinking the denominator. A scorecard can start with coverage as an improvement metric and promote it to a floor only once the denominator is stable.

A scorecard that tracks only coverage (for example, "X% of LSP methods are wired") is not a scorecard — it is a checklist. A scorecard has to have at least one metric that can get worse when code gets worse.

### What the minimum floor looks like per scorecard

The minimum-version rule needs one correctness floor, one real-user behavior floor, and one cost floor per scorecard. Below is the expected *shape* of each — exact thresholds live in the per-scorecard design issues and will settle when each scorecard is stood up. This table is an orientation aid, not a commitment.

| Scorecard | Correctness floor | Real-user floor | Cost floor |
|-----------|-------------------|-----------------|------------|
| Parser | Clean parse rate on the pinned corpus | Percent of top-1000 CPAN modules that produce zero parser errors | p99 parse time on a representative file |
| Diagnostics | Precision on the labelled idiom suite | False-positive rate on a clean real-world corpus | p99 diagnostic pass time per file |
| Editor intelligence | Goto-definition hit rate on labelled targets | Completion relevance on top-idiom files | p99 completion response time |
| Module resolution | Resolution correctness on the workspace / absolute / `use lib` / `FindBin` matrix | Cross-consumer consistency (all providers agree) | p99 resolution time |
| Workspace / indexing | Zero stale symbols after a scripted edit burst | Cold-start time on a 10k-file workspace | Peak index memory on same |
| DAP | End-to-end launch + attach + variables + evaluate success | Variables pane correctness on a scripted program | p99 evaluate round-trip |
| Engineering health | Banned-construct count (must stay at zero) | Flaky test count (must not grow) | p99 latency and peak memory roll-up |

A few notes on the table:

- "Correctness" is always on a **labelled** input: a corpus or suite where the expected answer is written down, so precision and recall are computable. Without labels there is no correctness metric, only a noise measurement.
- "Real-user" is always on a **realistic** input: a top-1000 CPAN module, a 10k-file workspace, a script someone might actually run under a debugger. It is the closest thing to a user-visible number.
- "Cost" is always a **tail** (p99 or peak), not a mean. Means hide the runs where a user rage-quits.

### Worked example: the #4107 catalog fix

[#4107](https://github.com/perl-lsp/perl-lsp/issues/4107) is the first public worked example of the ratchet model in action, applied to the DAP catalog:

- The **coverage** number (capability count) jumped from 102 to 116 because the catalog was corrected. That is a documentation fix, not a product improvement.
- Under the ratchet model, a coverage-only bump like that **cannot** raise a floor — only stable correctness, behavior, or cost metrics can. So the DAP scorecard ([#4069](https://github.com/perl-lsp/perl-lsp/issues/4069)) will track catalog coverage as an improvement metric, not a floor.
- The floor for DAP is expected to be the end-to-end scenario success rate (launch + attach + variables + evaluate on a known program), because that is a number that gets worse when debug logic gets worse.

The lesson generalises: **floors should live on metrics that degrade with real regressions**, not on metrics that move when we edit a catalog.

### Scorecard lifecycle: birth, update, retirement

Scorecards are not born complete. Each one moves through a short lifecycle, and the ratchet rules interact with that lifecycle in different ways depending on the stage:

1. **Proposed.** A design issue is filed (one of [#4063](https://github.com/perl-lsp/perl-lsp/issues/4063)–[#4070](https://github.com/perl-lsp/perl-lsp/issues/4070)). The scorecard does not exist on disk yet. Contributors should not tie PRs to it yet.
2. **Stood up, improvement-only.** The first PR creates `.ci/metrics/<subsystem>.json` with the four-question shape but **no floors**. Every metric starts as an improvement metric, because the denominator, suite, or corpus may still be churning. PRs can start referencing the scorecard at this stage.
3. **Minimum-version floors added.** Once the suite is stable, PRs begin promoting metrics from improvement to floor, one at a time. By the end of this stage the scorecard meets the minimum-version rule (one correctness floor, one real-user floor, one cost floor).
4. **Ratcheting.** Floors start moving up (or down, for cost metrics) via the stability window. This is the steady state. Most of a scorecard's life is spent here.
5. **Deprecated (rare).** A scorecard is retired only if the subsystem it owns is deleted or merged into another subsystem. Retiring a scorecard requires filing an issue against the umbrella [#4062](https://github.com/perl-lsp/perl-lsp/issues/4062) and is expected to be rare.

A few rules the lifecycle enforces:

- **Floors are additive.** New floors can be added at any time, but removing a floor requires a PR that explains why the old floor was wrong. Floors are not removed just because a refactor is inconvenient.
- **Corpus churn pauses the ratchet.** If the corpus a scorecard reads from changes (new files added, mis-labelled files fixed), the stability window restarts for any metric that depends on that corpus. This avoids locking in a number produced by a different input.
- **One scorecard is edited by one PR at a time.** Two concurrent PRs both raising a floor on the same scorecard will conflict at the JSON level. That is a feature — it forces serialisation on metric-file changes so one PR does not silently shadow another.

### What a scorecard file looks like in practice

The on-disk format is deliberately boring. Each scorecard is a single JSON object with a fixed shape — one block for floors, one for improvements, plus provenance. A stripped-down example, illustrative only and not the final schema, might look like this:

```jsonc
{
  "scorecard": "parser",
  "umbrella_issue": 4063,
  "recorded_at": "2026-04-11T12:34:56Z",
  "commit": "abc123def456",
  "floor": {
    "clean_parse_rate": { "value": 0.987, "direction": "up" },
    "corpus_green_lane":  { "value": 1.000, "direction": "up" },
    "p99_parse_ms":      { "value": 8.5,   "direction": "down" }
  },
  "improvement": {
    "node_kind_coverage": { "value": 0.82 },
    "recovery_salvage":   { "value": 0.61 }
  },
  "stability_window": {
    "runs_required": 5,
    "runs_observed": 2
  }
}
```

The rules this example encodes:

- Every metric is a named object with a `value`, not a bare number. That means a reader never has to guess units and a future schema extension (e.g. adding an error bar) does not break existing readers.
- Floor metrics carry a `direction` so the ratchet knows which way is "better". Cost metrics are `down`, correctness metrics are `up`.
- Improvement metrics live in a separate block. A future PR can promote one to a floor by moving it and adding `direction`.
- `recorded_at` and `commit` pin the measurement to a specific tree so an agent reading a stale file can detect drift.
- `stability_window` tells a ratchet whether this run is eligible to raise a floor. In the example, 2 out of 5 runs have been observed, so the floor cannot be raised yet.

The exact schema will be finalised when the first scorecard ships. The shape above is enough to reason about section 4's rules without having to open the schema issue.

## 5. Anti-patterns (do NOT ratchet these)

The following metrics are **not** product metrics and must not become public scorecards or floor gates. They live in internal orchestrator tooling (`.ops-perl-lsp/`), not in `.ci/metrics/`, and they do not belong on any of the seven scorecards:

| Anti-metric | Why it's not a product metric |
|-------------|-------------------------------|
| **PRs merged** | Measures throughput, not quality. A wave of one-line PRs looks identical to a wave of real features. |
| **Agents launched** | Internal execution noise. The number of agents used to produce a change says nothing about whether the change was good. |
| **Lines changed** | Rewards verbosity and penalises deletions and refactors. A well-placed one-line fix is worth more than a 200-line rewrite. |
| **Issues filed** | Measures scout throughput, not project health. You can file a hundred issues about the same root cause. |
| **`features.toml` entry count** | The #4107 case proved this is a catalog-shape metric, not a capability metric. Catalog corrections should not look like feature work. |

If a number answers "how hard did we work?" instead of "how good is the product?", it does not belong on a scorecard. Execution telemetry is fine — we need it — but it lives in the orchestrator's internal tooling so it cannot be mistaken for a product signal.

A practical test: if the metric can be improved **without touching the code the user runs**, it is probably an execution metric, not a product metric.

### Why these anti-patterns are tempting

Each of the anti-metrics exists because it is genuinely easy to measure and genuinely correlated with activity. PRs merged is a fine orchestrator heartbeat. Lines changed is a fine sanity check that a PR is not a typo fix labelled as a feature. Issues filed is a fine input to scout capacity planning. None of those uses are wrong.

What is wrong is **promoting an execution metric to a product floor**, because the execution metric is decoupled from whether the product actually works. A quarter in which PRs merged doubles but diagnostic precision drops is a bad quarter, not a good one, and the scorecard model is built so that "good" and "bad" are answerable from the floor set alone.

The rule is boring but firm: execution metrics stay in `.ops-perl-lsp/`, product metrics stay in `.ci/metrics/`. Never cross the streams.

## 6. How contributors should think about it

When you file an issue or open a PR, ask the following question first:

> **Which scorecard does this improve?**

If you cannot answer that in one sentence, one of two things is true:

1. **The work is out of scope.** For example, a one-off refactor with no correctness, latency, or coverage impact. That is not a reason to reject the work, but it is a reason to label it clearly as infrastructure / tooling / housekeeping rather than feature work.
2. **We are missing a scorecard.** If you keep running into work that does not belong to any of the seven, that is a signal to file a scout issue against [#4062](https://github.com/perl-lsp/perl-lsp/issues/4062) proposing a new scorecard.

### PR checklist

When you open a PR, the description should be able to answer these in one or two lines each:

- **Which scorecard?** (one of the seven, or "off-scorecard" with a reason)
- **Which metric on that scorecard?** (coverage / correctness / real-user / cost)
- **Is this a floor change or an improvement change?** A floor change must come with receipts — the new floor is only legal after the N-run stability window in layer 3 of the ratchet.
- **If off-scorecard, what's the justification?** (docs, infra, tooling, cleanup — all valid, just name it)

You do not need to literally fill out a form. You just need to be able to answer the questions if asked. A PR that cannot answer "which scorecard?" is either misclassified or not yet fully scoped.

### Routing examples

A few concrete examples of how common PR types map to scorecards:

| PR type | Primary scorecard | Primary metric | Notes |
|---------|-------------------|----------------|-------|
| Fix a parser panic on an obscure heredoc | Parser | clean-parse rate (floor) | If the panic was caught under `catch_unwind`, engineering health is also touched — file against parser, cross-reference engineering health. |
| Reduce completion false positives for `$self->` targets | Editor intelligence | completion relevance | Likely pairs with a diagnostics scorecard touch if the same fix improves lint precision. |
| Speed up initial workspace index on a 10k-file project | Workspace / indexing | cold-start time (floor) | Also touches engineering health (memory), but the user-visible improvement is workspace. |
| Add a DAP handler for `setExpression` | DAP | handler coverage (improvement) | Catalog coverage only, so it stays improvement-only until an end-to-end test exercises it. |
| Tighten a lint rule from "suggest" to "warn" | Diagnostics | precision (floor) | Any change to rule severity has to come with a precision receipt on the labelled corpus. |
| Clippy pass removing `unwrap()` from a leaf crate | Engineering health | banned-construct floor | Off-feature-scorecard; a clean engineering-health win. |
| Rewrite a status doc | Off-scorecard | N/A | Documentation. Not tied to any scorecard, but should still say "docs" in the PR description. |
| Add a new `features.toml` entry | Likely off-scorecard | N/A | If it is just a catalog entry, it is measurement, not product. Cross-reference the feature that justifies the entry. |

The table is not exhaustive — it is a set of exemplars. When in doubt, pick the scorecard that would *degrade* if the PR had a bug, not the one that would *improve* if the PR worked.

### Anti-example: "this PR improves all seven"

If you find yourself writing "this PR improves all seven scorecards", stop. Almost no single PR does that honestly. What is more likely is one of:

- The PR is actually seven unrelated changes and should be split.
- The PR is a refactor and is improving engineering health, not the feature scorecards.
- The PR is a catalog or schema change and is not improving any of the seven — the shape-not-scarcity insight again.

A good rule of thumb: a PR should improve **one scorecard primarily**, and may touch at most one or two others incidentally.

### Writing tests for scorecards

When you add a test that a scorecard will read, the test itself has to be deterministic enough to ratchet. Rules of thumb:

- **Prefer binary outcomes where possible** — "this test passes" is cheaper to ratchet than "this test's score is 0.78 ± 0.03".
- **If the metric is a distribution, record both the central value and a tail** — for example, p50 and p99 latency, not just mean.
- **Record the commit SHA inside the metric file** — so an agent reading a stale `.ci/metrics/<subsystem>.json` can detect staleness rather than trusting it.
- **Do not fold multiple scorecards into one test file** — a parser-scorecard test and a diagnostics-scorecard test should live in different files even if they happen to share a fixture.

### FAQ

**Q: My change is a pure refactor — no behavior change, no metric moved. Which scorecard?**

A: Probably none. Tag it as "off-scorecard, refactor". A refactor should leave every scorecard unchanged; that is the point. If your refactor *does* move a metric, that is interesting — say so in the PR description.

**Q: I want to add a metric that does not fit any of the seven. What do I do?**

A: First, check whether it fits the four-question shape of an existing scorecard. Most metrics do — for example, "number of `use lib` forms resolved" is a module-resolution coverage metric, not a new scorecard. If it truly does not fit, file a scout issue against the umbrella [#4062](https://github.com/perl-lsp/perl-lsp/issues/4062) proposing a new scorecard. Do not quietly add it to `.ci/metrics/misc.json`.

**Q: A CI run was much faster than usual — can we ratchet?**

A: No. The stability window (layer 3 of the ratchet) exists exactly for this case. A single fast run is a data point, not a trend. The ratchet will pick it up if it holds.

**Q: My PR regresses a floor. What are my options?**

A: You have three: (1) fix the regression so the floor holds; (2) explain in the PR why the floor should be lowered, with evidence that the old floor was wrong (e.g. it was measured on a biased corpus); or (3) split the PR so the regressing part lands under a feature flag. You should not "just lower the floor" without an argument for why the old floor was wrong.

**Q: Are the scorecards public-facing?**

A: The scorecard *files* live in the repo and anyone can read them. Whether and how they are published (e.g. on the docs site, on a dashboard, in release notes) is a separate decision owned by the release process, not by the scorecard design itself. Until then, `docs/project/status/` remains the canonical narrative surface and the scorecards are feed-in data for future status generation.

**Q: How does this relate to `features.toml`?**

A: `features.toml` is a **catalog**, not a scorecard. It answers "what LSP methods do we claim to implement?" — coverage-adjacent. The scorecards answer "how well do we implement them?" — correctness, real-user behavior, and cost. A feature is not "done" when it is listed in `features.toml`; it is done when the scorecard that owns it has a stable floor on it.

**Q: What happens to `docs/project/status/*.md`?**

A: Nothing. Those are generated per-subsystem status files that describe *what is true right now* and are rebuilt post-merge. They are a narrative surface aimed at readers who want a snapshot, and they do not conflict with the scorecards — in fact, future versions may pull directly from the scorecard files.

## 7. Cross-references

### Design tracking issues

| Issue | Title | Role |
|-------|-------|------|
| [#4062](https://github.com/perl-lsp/perl-lsp/issues/4062) | metrics: design layered scorecard model | **Umbrella issue.** Defines the 7-scorecard framing and the four-question shape. |
| [#4099](https://github.com/perl-lsp/perl-lsp/issues/4099) | metrics(research): reference-model findings | Survey of how rust-analyzer, gopls, pyright, and clangd expose metrics. |
| [#4105](https://github.com/perl-lsp/perl-lsp/issues/4105) | metrics(ratchet): 4-layer ratchet model | Defines the floor-vs-improvement split, the stability window, and the issue-to-scorecard rule. |

### Per-scorecard sub-issues

| Scorecard | Issue |
|-----------|-------|
| Parser | [#4063](https://github.com/perl-lsp/perl-lsp/issues/4063) |
| Diagnostics | [#4065](https://github.com/perl-lsp/perl-lsp/issues/4065) |
| Editor intelligence | [#4066](https://github.com/perl-lsp/perl-lsp/issues/4066) |
| Module resolution | [#4067](https://github.com/perl-lsp/perl-lsp/issues/4067) |
| Workspace / indexing | [#4068](https://github.com/perl-lsp/perl-lsp/issues/4068) |
| DAP | [#4069](https://github.com/perl-lsp/perl-lsp/issues/4069) |
| Engineering health | [#4070](https://github.com/perl-lsp/perl-lsp/issues/4070) |

### Historical context

- [#4107](https://github.com/perl-lsp/perl-lsp/issues/4107) — DAP catalog undercount. First worked example of the "underselling" pattern and the reason catalog coverage is an improvement metric rather than a floor metric.

### Related docs

- [docs/project/status/index.md](../status/index.md) — generated per-subsystem status surface (what is true *right now*).
- [docs/project/ROADMAP.md](../ROADMAP.md) — milestone view of what each scorecard is aiming at.
- [features.toml](../../../features.toml) — canonical LSP capability catalog. The catalog that the #4107 correction fixed.

---

**In one sentence**: the metric stack is seven scorecards that each answer coverage / correctness / real-user / cost, stored as machine-readable JSON, gated by a stability-window ratchet, with every open issue named against exactly one scorecard — so a contributor can always answer "which scorecard does this PR improve?".
