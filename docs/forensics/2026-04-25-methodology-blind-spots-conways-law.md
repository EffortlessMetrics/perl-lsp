# 2026-04-25 — Methodology Blind Spots: Conway's Law in the Orchestration Loop

**Lens**: Generalization of the tooling-debt observation into a structural principle about self-improving methodologies
**Purpose**: Name the failure mode where the methodology cannot improve what it cannot dispatch at, so future operators can detect and counter it deliberately

This doc is a meta-observation about the orchestration model itself, not about any specific PR cluster or failure mode. It generalizes a pattern that surfaced when filing #6791 / #6792 / #6793 / #6794 (tooling-debt issues) and extends to four other latent blind spots identified in this session.

---

## Core observation

The orchestration model dispatches agents at two granularities:

1. **PR granularity** — review, build, audit, merge
2. **Issue granularity** — scout, plan-review, build

What gets improved is determined by what the methodology can dispatch at. Anything that exists at PR or issue granularity gets improved cycle after cycle — that's where the verification ladder, the agent classes, the labels, and the routing logic all live. Anything that lives below or beside that granularity is structurally invisible to the improvement loop.

This is Conway's law operating on the methodology itself: the shape of what gets improved mirrors the shape of the dispatch layer. The methodology can only see problems that fit into a PR or an issue. Everything else gets routed around.

---

## The originating instance — tooling-debt

The xtask fmt false-cascade is the canonical example. The pattern:

- `xtask/src/tasks/fmt.rs` aborts on the first failing crate with a misleading message ("Failed to format `<crate>/Cargo.toml`")
- The same message appears across N independent PRs that each have their own per-PR fmt drift
- Result: every high-throughput session has multiple agents misclassify per-PR fmt drift as a master cascade
- Codified in `feedback_xtask_fmt_false_cascade.md`; 2026-04-25 calibration showed 7/12 flagged PRs were real fmt issues, the other 5 had different root causes masked by the same misleading message

Each occurrence costs ~5 minutes of premium-agent time (a master-bit-rot scout dispatch, a verification pass on master, sometimes a cascade-rebase wave that turns out to be unnecessary). Recurs ~4x per high-throughput session. ~20 minutes per session, every session, indefinitely until the underlying tool is fixed.

The methodology has the dispatch granularity to push a fmt commit to a PR. It does not have the dispatch granularity to "fix the underlying tool that produces misleading error messages." So it pushes the fmt commit, ships the PR, and the underlying friction recurs next session.

Filed 2026-04-25 against this exact failure mode:

- #6791 — xtask fmt error message
- #6792 — sandbox-fail-closed timeout
- #6793 — UX Regression Gate trigger
- #6794 — CARGO_BUILD_JOBS=1 phantom timeouts

These are the *symptoms*. The structural fix is not "file four issues." It is "create an agent class that can dispatch at sub-PR-level recurring friction." That class was added 2026-04-25 as the **tooling-debt-scout**.

---

## Four other instances of the same blind spot

The tooling-debt instance is one shape of the pattern. Four other shapes were identified during the 2026-04-25 retrospective:

### 1. The "almost ready" stuck-PR class

PRs that accumulated 4 of 5 sign-offs and got stuck on mechanical issues — rebase conflicts, force-push hygiene, fmt drift — for days. The verification ladder advances PRs through review labels but does not handle mechanical recovery. The cherry-pick wave on 2026-04-25 unblocked four such PRs (#6351, #6333, #6246, #5985); they had been sitting near-merge-ready for the better part of a week.

This is recurring. There is no agent class whose job is "detect near-merge-ready but mechanically blocked PRs at session start and unblock them." The verification ladder dispatches against issues and against PRs in active review states; it does not dispatch against the mechanical-recovery state.

The friction is at session-start scan granularity, not PR-review granularity. The methodology routes around it.

### 2. Memory recalibration drift

Forensics docs and memory entries have a half-life. The substrate moves underneath them — Codex 5.4 became Codex 5.5 in literal-yesterday wall-clock time. The "6.3% scout error rate" calibration in `feedback_research_verifier_roi.md` was measured against Codex 5.4 behavior. After the substrate shift, that number is wrong-by-default, but no agent class re-verifies it against current substrate.

Memory entries quietly drift into being confidently wrong. The methodology has dispatch granularity to write a new memory entry (any agent's wrap-up step can do that). It does not have dispatch granularity to "re-verify the empirical claims in existing memory entries against current substrate behavior."

Added 2026-04-25 (batch with this doc) as the **memory-recalibrator** agent class.

### 3. Cross-operator coordination

At single-operator scale, the orchestrator is the coordination layer. At multi-box / multi-repo / multi-operator scale, who decides which Codex bursts go where? Forensics docs partially solve cross-session coordination — a new operator can read the prior session's forensics and pick up routing decisions from real data. But there is no explicit tooling for cross-operator coordination *within* a session.

This will become acute at higher operator counts. It does not bite yet because operator count is one. The friction is at coordination-layer granularity, not PR or issue granularity. The methodology cannot currently dispatch at this granularity. Proposed but not yet built.

### 4. Spec-planner API surface enumeration

`Fix B` in `.spec/4513-red-tdd-api-read/` was deferred because it added spec-planner maintenance burden. The trade-off was made when no agent existed to enforce API-surface enumeration. With the three-way-match agent now existing, the trade-off should shift — but no scheduled re-evaluation exists.

Deferred decisions accumulate. The original deferral logic was sound; the conditions that made it sound have changed. The methodology has dispatch granularity to make new deferral decisions; it does not have dispatch granularity to "periodically re-evaluate prior deferrals against changed agent capabilities."

This is a different shape of the same blind spot: the friction is at decision-revisit granularity, not at the granularity of any individual decision.

### 5. Forensics index staleness

The `situation_id → fragment paths` index is what lets agents load the right context fast. If the index is not kept in sync as new forensics docs land, agents load outdated context. There is no agent class whose job is to maintain the index.

The friction is at index-maintenance granularity. The methodology can dispatch at "write a new forensics doc"; it cannot dispatch at "keep the index of existing docs current."

---

## The structural fix pattern

When a recurring friction is identified at sub-PR granularity, the right response is to **create a new agent class that dispatches at that granularity**. Not to file more individual issues against the symptoms. Not to add a checklist item to existing agent prompts (those rot). Not to write a memory entry that says "remember to handle this case" (also rots).

A new agent class has:

- A definition file (`.claude/agents/<name>.md`)
- A skill chain it executes
- Label conventions for its sign-off
- A routing rule in the orchestrator that knows when to dispatch it

The cost of creating a new agent class is real. It is also amortizable: once created, it executes every session indefinitely.

Examples instantiated 2026-04-25:

| Friction | Granularity | Agent class created |
|---|---|---|
| Tooling-debt (xtask fmt, etc.) | Sub-PR / cross-session | tooling-debt-scout |
| Memory entry drift | Per-memory-entry / cross-session | memory-recalibrator |

Examples proposed but not yet built:

| Friction | Granularity | Agent class needed |
|---|---|---|
| Stuck near-merge-ready PRs | Session-start scan | stuck-PR scout |
| Cross-operator coordination | Multi-operator session | coordination layer (design open) |
| Forensics index staleness | Per-doc-landing | forensics-index-keeper |
| Deferred-decision re-evaluation | Per-deferral / cross-session | deferral-revisit scout |

---

## Detection heuristic for blind spots

A friction is probably a methodology blind spot when **all** of the following hold:

1. The friction recurs across N sessions despite individual fixes (N ≥ 3 is a clear signal)
2. Each individual fix is at PR-level (push a commit, close a PR, add a label) rather than at root-cause level
3. No existing agent class names the friction as in-scope

The recurrence is the signal. If you find yourself thinking "we just fixed this last session" or "I keep telling agents to check for this," that is the heuristic firing.

The 2026-04-25 session retrospective applied this heuristic and found five blind spots. Two were addressed by creating new agent classes; three were filed as proposed-but-not-yet-built.

---

## Why the methodology has this shape

Dispatching agents at granularities other than PR or issue requires building new agent infrastructure: a definition file, a skill chain, label conventions, routing logic. PR/issue dispatch is the existing scaffolding. There is real friction to creating a new agent class — the orchestrator has to learn when to dispatch it, the labels have to not collide with existing ones, the skill chain has to compose with existing skills.

So the methodology defaults to "route around the friction at PR-level" because that is what the existing infrastructure supports. The default is not chosen; it falls out of the path of least resistance.

The choice is between paying the one-time cost of building the new agent class versus paying the per-session cost of routing around the friction forever. The math almost always favors the one-time cost — but only after someone explicitly compares them.

---

## The cumulative cost of unaddressed blind spots

Per-occurrence costs are small. Cumulative costs are large.

The xtask fmt false-cascade costs ~5 minutes of premium-agent time per occurrence and recurs ~4x per high-throughput session. ~20 minutes of premium-agent waste per session, every session, until the underlying tool is fixed. Across 50 sessions, that is ~16 hours of premium-agent time spent on a single avoidable failure mode.

The cost of creating the tooling-debt-scout agent class once: a few hours, paid down over those same 50 sessions. The crossover is fast.

This math holds for each of the five blind spots. The structural-fix pattern is not a luxury; it is the lower-cost option as soon as the recurrence count exceeds a small threshold.

---

## Operational protocol

Recommended additions to session retrospective:

- Ask explicitly: "what friction did we route around at PR-level that has a sub-PR root cause?"
- For each candidate, count occurrences across the last N sessions (use forensics docs as evidence)
- If a friction recurs across 3+ sessions: file an issue tagged `methodology-blindspot` and either (a) create a new agent class if one does not exist, or (b) extend an existing agent's scope if it fits naturally

Existing agent classes that address blind spots:

- **tooling-debt-scout** — recurring tool-level friction
- **memory-recalibrator** — drift in empirical claims in memory entries

Known-but-unaddressed blind spots (as of 2026-04-25):

- Stuck near-merge-ready PRs (proposed: stuck-PR scout)
- Cross-operator coordination (design open)
- Forensics index staleness (proposed: forensics-index-keeper)
- Deferred-decision re-evaluation (proposed: deferral-revisit scout)

---

## The meta-insight

A self-improving methodology can only improve along the granularities at which it can act.

Designing the methodology means choosing those granularities deliberately, not letting them default. Every granularity the methodology *cannot* dispatch at is a blind spot — a class of problem the methodology will route around forever. The defaults are PR and issue because that is where the existing scaffolding lives; absent deliberate intervention, those will remain the only granularities, and every problem that does not fit into them will accumulate.

The tooling-debt-scout's existence is not just about fixing four specific tools (#6791-#6794). It is about establishing a precedent that the methodology *can* dispatch at the meta-layer — that "the orchestration loop itself is improvable through the orchestration loop" is a thing that has been done at least once and can therefore be done again.

Each new agent class added at a previously-blind granularity makes the next one cheaper. The infrastructure-building cost is high for the first; the routing pattern, label convention, and skill-chain shape are reusable for the second; by the fifth, the marginal cost approaches the cost of creating any other agent class.

The methodology is not finished. It will never be finished. The work is to keep noticing the granularities at which it cannot currently see, and to deliberately add the dispatch capability that lets it see at those granularities.

---

## Applies-to

Apply this lens any time:

- An orchestrator notices "we keep having to do this manually"
- Evaluating whether to create a new agent class versus extend an existing one
- A recurring friction is identified and the proposed fix is at PR-level rather than root-cause level
- A session retrospective is being written and the question "what would prevent this next time?" is on the table
- Memory entries or forensics docs are being audited for currency
- The set of existing agent classes is being reviewed for coverage gaps
- Onboarding a new operator who needs to know what the methodology *cannot* currently see

Cross-references:

- `2026-04-25-failure-mode-catalog.md` — the failure modes that are visible to the methodology
- `2026-04-25-process-meta-learnings.md` — process patterns that operate within the existing dispatch granularities
- `2026-04-25-operator-playbook-templates.md` — wave shapes for the granularities that already exist
- `feedback_xtask_fmt_false_cascade.md` — the originating memory entry for the tooling-debt instance
- Issues #6791, #6792, #6793, #6794 — symptoms of the tooling-debt blind spot, filed 2026-04-25
