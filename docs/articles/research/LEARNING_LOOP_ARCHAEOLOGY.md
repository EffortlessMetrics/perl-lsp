# Learning Loop Archaeology
## How Incidents Become Durable Memory In This Repo

This repository does not treat mistakes as dead ends. It turns them into a
loop:

1. something goes wrong or looks suspicious
2. the failure is isolated with receipts
3. the result is written down in a durable surface
4. the best examples are promoted into exhibits
5. future work consults that memory before repeating the mistake

That is the real learning system here. It is bigger than any one file, and it
is one reason this codebase is unusually legible after the fact.

---

## 1. Wrongness Is Logged, Not Hidden

[`docs/project/LESSONS.md`](../../../docs/project/LESSONS.md) is the repo's
wrongness ledger. It uses a fixed pattern:

- wrong
- evidence
- fix
- prevention

That structure matters more than the specific entries. It means the repo keeps
failure in a falsifiable form instead of collapsing it into a vague postmortem.
The lessons file is where drift, flakiness, and claim errors become reusable
engineering knowledge.

The learning categories are also meaningful:

- claim drift
- measurement drift
- harness drift
- scope drift
- non-determinism
- coverage illusion
- packaging drift

Those are not generic bug labels. They are the vocabulary the repo uses to
remember what went wrong and what should not happen again.

---

## 2. Forensics Turns Incidents Into Evidence

[`docs/forensics/`](../../../docs/forensics/) is the evidence pipeline. It is
where PR facts are harvested, dossiered, and promoted into stronger forms of
memory.

The production steps are explicit:

- `pr-harvest` collects metadata, commits, files, and review-thread facts
- `dossier-runner` orchestrates the analysis pipeline
- `render-dossier` produces the final dossier or exhibit artifact

The repo's own forensics prompts make the same point more strongly:

- `measurement-auditor` rejects unstable or dishonest comparisons
- `policy-auditor` checks schema drift, catalog drift, and guardrail drift

That means the learning loop is not just "remember the bug." It is:

- classify the bug
- verify the evidence
- prevent the recurrence
- preserve the result in a reusable artifact

---

## 3. Casebook Promotes The Best Examples

[`docs/project/CASEBOOK.md`](../../../docs/project/CASEBOOK.md) is not a
dumping ground for notable PRs. It is an exhibit system.

Each exhibit records:

- what it proves
- the review map
- the proof bundle
- the scar story, if there was one
- quality deltas
- budget with provenance

That design matters because it promotes learning from incident history into
teachable examples. The casebook is where a repaired problem becomes a
reference implementation for future work.

The early exhibits also show the repo's bias toward traceability:

- explicit PR numbers
- explicit proof bundles
- explicit scar stories
- explicit provenance

That keeps the casebook from becoming anecdotal. It stays tethered to GitHub
history and the repo's own receipts.

---

## 4. GitHub Becomes A Memory Graph

The issue tracker and PR ledger are not just workflow plumbing. They are part of
the learning system.

The crosslink pattern is the important part:

- issue -> PR
- PR -> follow-up issue
- PR -> learning issue
- PR -> article issue

Examples from the archive:

- [issue #157](https://github.com/EffortlessMetrics/perl-lsp/issues/157)
  remembers the review outcome of PR `#153`
- [issue #198](https://github.com/EffortlessMetrics/perl-lsp/issues/198)
  turns PR `#176` into stabilization follow-up work
- [issue #1667](https://github.com/EffortlessMetrics/perl-lsp/issues/1667)
  and [issue #1678](https://github.com/EffortlessMetrics/perl-lsp/issues/1678)
  preserve swarm-operational lessons
- [issue #2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
  and [issue #2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)
  preserve parser-fix learning reports
- [issue #2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195)
  and [issue #2197](https://github.com/EffortlessMetrics/perl-lsp/issues/2197)
  turn implementation history into publication evidence

That is why later sessions can often recover a story from GitHub itself instead
of relying on chat logs.

---

## 5. Swarm-State Stores Operational Memory

[`.claude/swarm-state/README.md`](../../../.claude/swarm-state/README.md)
defines the control-plane memory layout:

- `discovered-issues.md` for leads noticed during other work
- `known-pitfalls.md` for reusable traps and lessons
- `completed-slices.md` for dedup and lifecycle status
- `swarm-queue.json` for active overlap and ownership
- `findings.json` for durable control-plane conclusions

This is the repository learning loop in operational form. It is not just
remembering failures. It is remembering:

- what was already tried
- what was learned
- what is still live
- what should carry forward into the next session

That makes the swarm state a memory layer, not a scratchpad.

---

## 6. The Loop Is Self-Reinforcing

The strongest thing about this repo is that each learning surface feeds the
others:

- lessons record wrongness
- forensics turns evidence into dossiers
- casebook promotes the best examples
- swarm-state preserves operational memory
- issues and PRs keep the graph recoverable
- article issues turn the same evidence into public narrative

The result is a codebase that can learn from its own incidents without losing
the underlying chain of evidence.

That is a rare property. It means the repo is not only building software. It is
building a durable memory system for how the software, the swarm, and the
maintainer process evolve over time.

---

## Evidence Pointers

- [`docs/project/LESSONS.md`](../../../docs/project/LESSONS.md)
- [`docs/project/CASEBOOK.md`](../../../docs/project/CASEBOOK.md)
- [`docs/project/AGENTIC_DEV.md`](../../../docs/project/AGENTIC_DEV.md)
- [`docs/forensics/README.md`](../../../docs/forensics/README.md)
- [`docs/forensics/INDEX.md`](../../../docs/forensics/INDEX.md)
- [`docs/forensics/prompts/measurement-auditor.md`](../../../docs/forensics/prompts/measurement-auditor.md)
- [`docs/forensics/prompts/policy-auditor.md`](../../../docs/forensics/prompts/policy-auditor.md)
- [`.claude/swarm-state/README.md`](../../../.claude/swarm-state/README.md)
- [`.claude/swarm-state/findings.json`](../../../.claude/swarm-state/findings.json)
- [`.claude/swarm-state/findings.schema.json`](../../../.claude/swarm-state/findings.schema.json)
- [issue #157](https://github.com/EffortlessMetrics/perl-lsp/issues/157)
- [issue #198](https://github.com/EffortlessMetrics/perl-lsp/issues/198)
- [issue #1667](https://github.com/EffortlessMetrics/perl-lsp/issues/1667)
- [issue #1678](https://github.com/EffortlessMetrics/perl-lsp/issues/1678)
- [issue #2190](https://github.com/EffortlessMetrics/perl-lsp/issues/2190)
- [issue #2191](https://github.com/EffortlessMetrics/perl-lsp/issues/2191)
- [issue #2195](https://github.com/EffortlessMetrics/perl-lsp/issues/2195)
- [issue #2197](https://github.com/EffortlessMetrics/perl-lsp/issues/2197)
