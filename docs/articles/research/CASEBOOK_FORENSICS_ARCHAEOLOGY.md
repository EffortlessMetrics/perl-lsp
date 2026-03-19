# Casebook Forensics Archaeology
## How Scar Stories Became A Committed Memory System

This repository did not leave postmortems as loose notes. It turned them into a
repeatable evidence pipeline: PR facts are harvested, dossiers are rendered,
lessons are logged, and the best examples are promoted into casebook exhibits.
That is the historical seam worth preserving here.

The interesting part is that the repo made postmortem knowledge reusable in two
directions at once:

- forward, as a control surface for future PRs and audits
- backward, as a memory layer that explains why earlier choices existed

---

## 1. The Casebook Turned Good PRs Into Exhibits

[`docs/project/CASEBOOK.md`](../../../docs/project/CASEBOOK.md) is not just a
list of notable changes. It is an exhibit system. Each entry records:

- what the PR proves
- the review map
- the proof bundle
- the scar story, if there was one
- quality deltas
- budget with provenance

That structure matters. It means the repo does not treat wrongness as shameful
noise or only store clean successes. It preserves the chain from failure to
fix to prevention, then packages the result as a reusable example.

The file first appeared in `25f0b29a5` as part of the initial forensics and
lessons ledger work. A later architecture PR (`ea09be351`) shows the casebook
already being used as a living exhibit surface, before the Diataxis move placed
it under `docs/project/` on `e26d45d79`.

That relocation is part of the story. The casebook became governance, not a
miscellaneous doc.

---

## 2. The Forensics Directory Became A Dossier Pipeline

[`docs/forensics/`](../../../docs/forensics/) is the production side of the
memory system. Its README describes two linked workflows:

- pre-PR work orders
- post-PR dossiers

The inventory in [`docs/forensics/INDEX.md`](../../../docs/forensics/INDEX.md)
adds a clear maturity ladder:

- Level 0: inventory
- Level 1: dossier
- Level 2: exhibit

That is the crucial design choice. Raw PR history is not enough. It gets
classified, scored, and either left as inventory or promoted into casebook
form.

The scripts confirm that this was meant to be operational, not ceremonial:

- [`scripts/forensics/pr-harvest.sh`](../../../scripts/forensics/pr-harvest.sh)
  extracts PR metadata, commits, files, and review thread facts
- [`scripts/forensics/dossier-runner.sh`](../../../scripts/forensics/dossier-runner.sh)
  orchestrates harvest, temporal analysis, telemetry, and render steps
- [`scripts/forensics/render-dossier.sh`](../../../scripts/forensics/render-dossier.sh)
  synthesizes the final markdown dossier or exhibit cover sheet

The directory README says the output feeds dossier creation, and the scripts are
idempotent. That makes forensics a repeatable build artifact, not a one-off
investigation.

---

## 3. Specialist Auditors Made Evidence Honest

The prompt pack in [`docs/forensics/prompts/`](../../../docs/forensics/prompts/)
shows the repo hardening its own memory layer.

[`measurement-auditor.md`](../../../docs/forensics/prompts/measurement-auditor.md)
is the strongest signal. It treats metrics as claims that must survive:

- provenance checks
- reproducibility checks
- delta correctness checks
- theater detection

It also hard-fails comparisons when the measurement contract is unstable or
unknown. That is the opposite of generic QA. It is a policy for not publishing
false certainty.

[`policy-auditor.md`](../../../docs/forensics/prompts/policy-auditor.md) does a
similar job for governance. It checks catalog drift, metrics drift, schema
compliance, and guardrail effectiveness. Together, the auditors make the
forensics layer self-policing.

This is the point where the repo stops merely “documenting lessons” and starts
enforcing evidence discipline around them.

---

## 4. Lessons Became A Durable Wrongness Ledger

[`docs/project/LESSONS.md`](../../../docs/project/LESSONS.md) is the
institutional memory of failure. It uses a fixed pattern:

- wrong
- evidence
- fix
- prevention

That is a strong boundary. It prevents scar stories from collapsing into vague
war stories. Each entry is falsifiable, linked to ground truth, and paired with
a systemic prevention step.

The categories themselves show the repo's memory model:

- claim drift
- measurement drift
- harness drift
- scope drift
- non-determinism
- coverage illusion
- packaging drift

Those are not random bug labels. They are the vocabulary the repo uses to
remember how trust failed and how to stop that failure from recurring.

---

## 5. The Operating Model Is Receipt-First

[`docs/project/AGENTIC_DEV.md`](../../../docs/project/AGENTIC_DEV.md) explains
the broader posture: claims are receipt-based, wrongness is recorded, and
mechanical gates are more trustworthy than human memory.

The forensics system is how that posture gets applied to history:

- `pr-harvest` gathers raw facts
- the analyzers classify and score them
- `measurement-auditor` blocks dishonest comparison
- `policy-auditor` checks governance surfaces
- the dossier becomes the durable record
- the casebook promotes the best examples into exhibit form

That is why this codebase feels unusual. It does not just have docs about how
it works. It has a committed memory system that turns scar stories into
reusable operating evidence.

---

## Evidence Pointers

- [`docs/project/CASEBOOK.md`](../../../docs/project/CASEBOOK.md)
- [`docs/project/LESSONS.md`](../../../docs/project/LESSONS.md)
- [`docs/project/AGENTIC_DEV.md`](../../../docs/project/AGENTIC_DEV.md)
- [`docs/forensics/README.md`](../../../docs/forensics/README.md)
- [`docs/forensics/INDEX.md`](../../../docs/forensics/INDEX.md)
- [`docs/forensics/prompts/measurement-auditor.md`](../../../docs/forensics/prompts/measurement-auditor.md)
- [`docs/forensics/prompts/policy-auditor.md`](../../../docs/forensics/prompts/policy-auditor.md)
- [`scripts/forensics/pr-harvest.sh`](../../../scripts/forensics/pr-harvest.sh)
- [`scripts/forensics/dossier-runner.sh`](../../../scripts/forensics/dossier-runner.sh)
- [`scripts/forensics/render-dossier.sh`](../../../scripts/forensics/render-dossier.sh)
- `25f0b29a5`, `ea09be351`, `e60b46076`, `e26d45d79`, `156b9f44f`
