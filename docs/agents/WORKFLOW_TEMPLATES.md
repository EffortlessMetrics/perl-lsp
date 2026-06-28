# Workflow Templates

Outline-level templates for the six queue-processing workflows. These are for
recurring queue operations — not one-off fixes. One-off fixes follow the
standard scout→plan-review→build→review pipeline in CLAUDE.md.

Cross-reference:
- [ORCHESTRATION_ROLES.md](ORCHESTRATION_ROLES.md) — model-tier allocation per role
- [SCOUT_PROMPTS.md](SCOUT_PROMPTS.md) — ready-to-paste scout prompts for each phase
- [EVIDENCE_STANDARD.md](EVIDENCE_STANDARD.md) — required artifacts per claim type

---

## Workflow 1 — pr-classifier

**Purpose:** Batch-classify a queue of open PRs into merge/port/close-superseded/defer/duplicate.

**Batch size:** 10 PRs per wave. Never classify more than 10 in one agent run —
classification quality degrades and adversarial-pass coverage thins.

**Phases:**

| # | Phase | Model | Output artifact |
|---|-------|-------|-----------------|
| 1 | Fetch queue | haiku | JSON list of open PR numbers with titles |
| 2 | Classify (batch of 10) | haiku (Template 1) | `target/reconciliation/wave-N-classifications.json` |
| 3 | Adversarial second pass on close/defer | haiku (Template 2/3) | Annotated classifications — any close/defer must survive challenge |
| 4 | Route survivors | orchestrator | Label updates, queue state update |

**Adversarial second pass rule:** Every PR classified as `close-superseded` or
`defer` in phase 2 must be independently re-evaluated in phase 3 using the
verify-reachability or verify-duplicate prompt. If the second pass disagrees,
escalate to sonnet reviewer — do not resolve the disagreement at haiku tier.

**Output:** `target/reconciliation/wave-N-classifications.json` — one ledger row per PR.
Schema: [pr-ledger.schema.json](pr-ledger.schema.json).

---

## Workflow 2 — issue-triage

**Purpose:** Route new or untriaged issues to the correct pipeline stage.

**Phases:**

| # | Phase | Model | Output artifact |
|---|-------|-------|-----------------|
| 1 | Classify issue type | haiku (Template 6) | Classification JSON |
| 2 | Accuracy check (file paths, function names) | haiku (accuracy-scout) | Corrected facts |
| 3 | Route to plan-review (if bug/feature) | orchestrator | `needs-plan-review` label applied |

**Evidence rules for "already-fixed" routing:**
- Provide commit SHA on main that fixes it.
- Run merge-base check; paste output.
- Do not route to close without both.

**Output:** Labeled issue with routing decision. No ledger file — issues use
GitHub label state as the output artifact.

---

## Workflow 3 — source-swarm-reconciliation

**Purpose:** Align changes between the source repo and the swarm repo, resolving
divergence from parallel development.

**Phases:**

| # | Phase | Model | Output artifact |
|---|-------|-------|-----------------|
| 1 | Diff source vs swarm main | haiku | `target/reconciliation/source-diff-summary.md` |
| 2 | Classify each divergence | haiku (Template 1) | Ledger rows with `sync_direction` |
| 3 | Port swarm->source candidates | builder (sonnet) | Port PRs opened against source repo |
| 4 | Port source->swarm candidates | builder (sonnet) | Port PRs opened against swarm repo |
| 5 | Verify convergence | haiku (verify-reachability) | Confirmation that synced commits are in ancestry |

**sync_direction values (from pr-ledger schema):**
- `swarm->source`: swarm has a fix the source needs.
- `source->swarm`: source has a fix the swarm needs.
- `none`: divergence is intentional or cosmetic.

**Output:** Ledger rows in `target/reconciliation/` + opened port PRs.

---

## Workflow 4 — release-readiness

**Purpose:** Determine whether a release is ready to cut, and if not, what blocks it.

**Phases:**

| # | Phase | Model | Output artifact |
|---|-------|-------|-----------------|
| 1 | Ancestry check — release branch vs main | haiku (Template 3) | Reachability proof |
| 2 | Tree check — expected files present | haiku (Template 5) | Audit findings |
| 3 | Version check — Cargo.toml + CHANGELOG | haiku | Version consistency report |
| 4 | Receipts check — gate labels present | haiku | Missing-receipt list |
| 5 | Channel claims — docs + public announcements | haiku (Template 5) | Claims accuracy report |
| 6 | Dispatch recommendation | orchestrator | RELEASE-READY | BLOCKED (with blockers list) |

**Blocker criteria (from Template 7):**
- Correctness: data loss, crash, wrong output.
- CI gate: a required check is failing on main.
- API contract: a published interface has broken changes without a semver bump.

**Output:** `target/reconciliation/release-readiness-VVERSION.md` with RELEASE-READY
or BLOCKED verdict and evidence table.

---

## Workflow 5 — ci-failure-cluster

**Purpose:** Resolve a cluster of related CI failures across multiple PRs without
fixing each independently.

**Phases:**

| # | Phase | Model | Output artifact |
|---|-------|-------|-----------------|
| 1 | Read failure logs (batch) | haiku (Template 4) | Failure JSON list |
| 2 | Cluster by root cause | haiku | Cluster map (root cause → affected PRs) |
| 3 | Identify single root cause (must be ONE) | haiku | Root-cause statement with evidence |
| 4 | Fix root cause | builder (sonnet) | Fix PR on the failing branch or main |
| 5 | Re-run exact gate | CI / pr-responder | Gate result (pass/fail) |

**One-root-cause rule:** If clustering produces more than one independent root
cause, split into two separate workflow runs. A cluster fix that touches two
independent root causes is a scope violation.

**Output:** Fix PR + re-run result. Affected PRs are unblocked or re-routed.

---

## Workflow 6 — ub-review-calibration

**Purpose:** Calibrate the undefined-behavior review signal by classifying false
positives and tuning scout profiles.

**Phases:**

| # | Phase | Model | Output artifact |
|---|-------|-------|-----------------|
| 1 | Collect recent UB scout reports | haiku | Report list with item IDs |
| 2 | Classify each: TP / FP / quiet / infra | haiku | Classification list |
| 3 | Profile tuning — adjust haiku scout prompts | sonnet | Updated prompt delta |
| 4 | Upstream gaps — UB findings not yet fixed | orchestrator | Follow-up issue list |

**Classification definitions:**
- **TP (true positive):** Real UB or soundness issue caught by the scout.
- **FP (false positive):** Scout flagged safe code; scout prompt needs tightening.
- **quiet:** Scout should have flagged but did not; coverage gap.
- **infra:** Scout failed due to CI infra issue, not a content decision.

**Output:** Tuned scout prompt delta (reviewed by sonnet before applying) +
follow-up issues for upstream gaps.
