# Issue Discovery / Bug Scout Desk

This is the canonical doctrine for the **Issue Discovery / Bug Scout Desk**
lane — the swarm's radar. It is upstream of the Issue Research / Plan
Review Desk and answers one question:

> **Where are the next real issues hiding?**

Its job is *not* to fix, *not* to plan deeply, and *not* to flood GitHub.
Its job is to find high-signal candidate defects, gaps, and risks from the
repo's actual surfaces, then file or hand off only the ones with evidence.

This doc complements [`LANE_BOUNDARIES.md`](LANE_BOUNDARIES.md) (lane
ownership), [`PIPELINE_GATES.md`](PIPELINE_GATES.md) (the 7-gate pipeline),
and [`ORCHESTRATION_DOCTRINE.md`](ORCHESTRATION_DOCTRINE.md) (routing
rationale). The discovery desk sits *before* Gate 1 (Identify): it produces
the raw material the scout/plan-review pipeline refines.

## Position in the swarm

```text
Issue Discovery Desk      ← this lane (find suspicious seams, file candidate packets)
  → Issue Research / Plan Review Desk   (verify, scope, dedupe → builder-ready)
    → Builder lane                       (implement scoped fixes → PRs)
      → PR review / merge lane           (land green, reviewed PRs)
```

Operationally:

```text
find suspicious seams → verify they are not noise → file candidate issue
  → plan-review → build → merge
```

This lane should run continuously in the background, like radar.

## Difference from the plan-review lane

| Lane | Primary job | Output |
|------|-------------|--------|
| **Issue Discovery / Bug Scout Desk** | Find new candidate issues | Evidence packets / candidate issues |
| **Issue Research / Plan Review Desk** | Improve and verify issue plans | Builder-ready issues |
| **Builder lane** | Implement scoped fixes | PRs |
| **Merge lane** | Land green, reviewed PRs | Mainline changes |

The discovery lane does **not** polish issue plans. It finds real seams,
files concise candidate issues, and passes them forward. Deep planning is
the next desk's job.

## Mission

Find real, actionable issue candidates across `perl-lsp-swarm`:

```text
bugs · panic surfaces · protocol inconsistencies · test gaps · stale docs
bad assumptions · flaky behavior · missing CI guards · parser coverage gaps
LSP/DAP edge cases · workspace-facts blind spots · UX failure paths
agent/tooling failure modes
```

The output is a **candidate issue packet** — not a giant issue body, not a
builder-ready plan. A good packet contains enough evidence that the
plan-review lane can verify it without starting from zero.

## Core operating rule

### Discovery can batch. Filing cannot.

Read-only discovery may run wide:

```text
grep · static inspection · changed-file comparison · test inventory
CI/check snapshots · docs/source mismatch review · protocol surface review
receipt/status review · coverage gap review
```

But **mutating actions are issue-by-issue**:

```text
file issue · label issue · close issue · mark duplicate · edit issue body
promote issue
```

This is the same PR-by-PR discipline that protected the Codex backlog from
a bad curator verdict: shared base commits and shared helper files *looked*
like redundancy, but changed-file inspection proved most PRs were distinct
test surfaces needing sequencing, not closure. Aggregate to discover;
act one item at a time.

## Core doctrine — find issues by evidence, not vibes

A scout may *think* "this looks suspicious." It should only **file** when it
can say:

```text
Here is the source surface.
Here is the example.
Here is why current behavior is wrong or risky.
Here is how to verify it.
Here is why it is not already covered by an existing issue.
```

Optimize for **few, strong findings**. Volume is not the metric (see
[Metrics](#metrics)).

## Candidate issue packet

Use this lightweight packet — the GitHub form lives at
[`.github/ISSUE_TEMPLATE/candidate_issue.yml`](../../.github/ISSUE_TEMPLATE/candidate_issue.yml).

```md
## Finding
One-sentence description.

## Evidence
- Source:
- Test / fixture:
- Receipt / CI / docs:
- Related issues / PRs:

## Impact
Who sees this (user / maintainer / CI / tooling) and how?

## Minimal reproduction / sequence
command · Perl snippet · LSP request sequence · DAP request sequence · workflow case

## Suspected root area
- File:
- Function / type:
- Boundary:

## Why this is not already covered
Checked: open issues · recent merged PRs · open PRs · tests

## Suggested next workflow
needs-repro · needs-plan-review · needs-architecture-review ·
needs-source-grounding · direct small builder · discard if disproven

## Confidence
high / medium / low
```

Do **not** overbuild the body. The next desk handles full builder-ready
planning.

## Scout waves

Nine discovery surfaces. The **first wave** (1–6) covers the recently hot
seams; hold UX and workspace-facts (7–8) and test-quality (9 — folded into
the others) for wave two unless there is spare capacity.

| # | Scout | Surface | Finds |
|---|-------|---------|-------|
| 1 | `scout-find-dap-gaps` | DAP stack/scopes/variables/lifecycle/transport | weakened assertions, stale frame/variable refs, stackTrace line drift, malformed responses, lifecycle order bugs |
| 2 | `scout-find-lsp-gaps` | LSP document state, providers | stale `didChange`, URI isolation bugs, completion noise, hover false precision, code actions that appear but can't apply |
| 3 | `scout-find-parser-gaps` | parser / AST / `NodeKind` | wrong AST shape, missing fixtures for reachable variants, recovery-node overuse, valid syntax rejected, invalid syntax silently accepted |
| 4 | `scout-find-ci-ops-gaps` | `.github/workflows`, cleanup, gates | bare self-hosted routing, missing capacity labels, path-filter holes, stale labels, check-name drift, cleanup blind spots |
| 5 | `scout-find-robustness-gaps` | parser/lexer/LSP/DAP request surfaces | panic surfaces, DoS, unchecked indexing, byte-boundary slicing, unbounded recursion/buffers, `unwrap`/`expect` in server paths |
| 6 | `scout-find-docs-receipt-drift` | `docs/project/status/**`, receipts | stale counts, wrong source-of-truth, basis conflicts, docs claiming completion without receipt support |
| 7 | `scout-find-workspace-facts-gaps` | module resolver, package facts | static-resolution gaps, dynamic misclassified as certain, multi-root bugs |
| 8 | `scout-find-editor-ux-gaps` | `vscode-extension/**` | activation misses, silent startup failure, diagnostics with no next step, quick-fixes that can't apply |
| 9 | test-quality (cross-cutting) | `tests/**`, fixtures | tests that assert only "does not panic", assertions weakened to `> 0`, smoke tests that hide stale state |

### Why these first

| Scout | Reason |
|-------|--------|
| DAP | Active seam — recent #766/#768/#927 findings |
| LSP | Recent smoke-test cluster and stale-state risk (#757) |
| Parser | High leverage; the `NodeKind` lane exposed useful surfaces |
| CI/Ops | Swarm throughput depends on it |
| Robustness | High-confidence, often small fixes |
| Docs/receipts | Prevents stale dashboards from driving wrong work |

### Anchoring examples (why evidence beats vibes)

- **DAP breakpoint fork.** The correct discovery was not "tests are flaky";
  it was "line 4 vs 5 exposes a real adapter/stackTrace basis issue, and
  #766/#768 must be reconciled by evidence." A test weakened from an exact
  line assertion to `frame_line > 0` would have *hidden* the defect — that
  is the canonical test-quality finding this lane exists to catch.
- **Runner routing.** The load-bearing fault was not "a CX33 runner is
  weak"; it was that heavy jobs used **bare self-hosted routing**, making
  them eligible for the tiny pool. The durable answer was repo routing plus
  runner-side labels/groups, rolled out warn-only before enforcement.
- **LSP smoke cluster.** Previous smoke PRs touched the same file but were
  *complementary*; discovery must classify that as "sequence both," not
  "duplicate."
- **Receipt basis conflict.** A seam-inventory basis vs a modern `ripr+`
  canonical actionable count is not a mechanical merge conflict — it is a
  **basis conflict** that needs discovery and reconciliation, not a silent
  pick.

The workspace-facts rule is its own anchor:

> Unknown is acceptable. Pretending dynamic Perl is statically known is not.

## Confidence levels → filing rules

| Confidence | Action | Requirements |
|------------|--------|--------------|
| **high** | File directly | source evidence · minimal example/sequence · clear user/CI/tooling impact · not already covered · specific suspected area |
| **medium** | File as `needs-research`, or hand to plan-review / source-grounding | strong smell · partial evidence · clear next verification step |
| **low** | Do **not** file | record in scout report; note what would raise confidence |

Per scout, per wave:

```text
max 5 candidate packets
max 2 filed issues — unless findings are clearly high-confidence
```

Prefer **updating an existing issue** over filing a duplicate.

## Labels

The lane uses functional labels. It must **never** apply `builder-ready` —
that belongs to the plan-review lane.

```text
candidate-issue · needs-research · needs-repro · needs-source-grounding
needs-plan-review · needs-architecture-review
bug · test-gap · docs-drift · ci-ops · lsp · dap · parser
workspace-facts · ux · robustness · tooling-debt
```

The candidate-issue template applies `candidate-issue` + `swarm-discovered`
on entry. Triage (below) adds the routing label that hands the packet to
the next desk.

## Deduping rules

**Never** call something a duplicate from:

```text
same broad theme · same file · same helper · same base commit
similar diffstat · a curator summary
```

**Do** dedupe on:

```text
same failure mode · same source surface · same user-visible behavior
same intended fix · same acceptance test
```

This is the discipline that kept the Codex backlog intact. Other agent
outputs (curator summaries, prior verdicts) are *leads*, not facts —
verify from source or a primary artifact before filing or deduping.

## Triage pass

Do not build directly from findings. After a wave, run a short triage. For
each candidate pick exactly one next lane:

```text
keep · merge into existing issue · send to plan-review desk
send to architecture review · send to repro-lab · discard as noise
```

Example triage table:

| Candidate | Confidence | Duplicate? | Next lane |
|-----------|-----------:|------------|-----------|
| DAP stale variableReference | high | no | plan-review |
| Parser fixture missing | medium | maybe #356 | source-grounding |
| Docs stale count | high | no | direct docs builder |
| UX hover false precision | low | unknown | hold |

## Guardrails

**No issue flooding.** Max 5 candidate packets and max 2 filed issues per
scout unless clearly high-confidence. The lane wins by signal quality.

**No destructive action.** Discovery agents must not: close issues, retitle
PRs, remove labels, rebase branches, push code, open PRs, mark
builder-ready, or merge/rebase anything. The single permitted mutation is
**filing or updating a candidate issue**, one at a time.

**No "curator says so."** Treat other agent outputs as leads. Verify from
source or primary artifact before filing.

**No high-frequency GitHub polling.** Use point-in-time snapshots (issue
search, PR file list, check-run snapshot, workflow file inspection). Do not
set GraphQL watchers or poll on a tight loop — that rule is already part of
the maintainer-agent doctrine.

## Metrics

Track:

```text
candidate issues found · discarded · merged into existing
promoted to plan-review · later promoted to builder-ready
false-positive rate · duplicate rate · mean time finding → builder-ready
```

The most important metric is **not volume**. It is:

```text
percentage of filed findings that survive plan review
```

If that number is low, scouts are filing too eagerly.

## Tooling roadmap (deferred)

Build tooling only **after** the lane proves useful. Sequenced as
report-only / no-mutation surfaces first:

1. **Candidate template + doctrine** — `candidate_issue.yml` + this doc. *(done)*
2. **Discovery report** — `cargo xtask issue-discovery report`: open
   candidates, `needs-repro`, high-confidence unplanned, stale candidates,
   duplicates.
3. **Source-surface grep packs** — `grep-panic-surfaces`,
   `grep-dap-invalid-refs`, `grep-lsp-stale-state`,
   `grep-workflow-bare-self-hosted` (report-only).
4. **Scout output validator** — assert packets include evidence, impact,
   suspected area, dedupe notes, confidence, next workflow.
5. **Candidate-to-plan handoff** — `cargo xtask issue-discovery handoff
   ISSUE` generates a plan-review checklist from a candidate.

## What this lane prevents

Without it, the backlog is limited to what broke loudly, what a user
noticed, what a builder tripped over, or what a scout happened to file
during another lane. With it, the swarm develops peripheral vision: hidden
protocol bugs, weak tests, stale docs, unsafe fallbacks, bad CI
assumptions, silent UX failures, and parser gaps — *before* users hit them.

The governing constraint is the one that recurs across the swarm:

> **Finding is cheap; being right is expensive.**

This lane spends just enough effort to make findings worth plan-review, and
no more — scouts are radar, not builders.

## See also

- [`LANE_BOUNDARIES.md`](LANE_BOUNDARIES.md) — lane ownership and the
  non-overlap rule.
- [`PIPELINE_GATES.md`](PIPELINE_GATES.md) — the 7-gate pipeline this lane
  feeds.
- [`CLUSTER_CURATION.md`](CLUSTER_CURATION.md) — dedupe-by-evidence
  discipline for external-agent PR clusters.
- [`.github/ISSUE_TEMPLATE/candidate_issue.yml`](../../.github/ISSUE_TEMPLATE/candidate_issue.yml)
  — the candidate packet form.
- `.claude/agents/scout-find-*.md` — the six discovery scout definitions.
