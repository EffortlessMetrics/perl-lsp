# Guidance Style — Compiler Discipline, Coach Output

**Status**: Advisory reference doc, not an enforced gate. Nothing in this repo's
CI, labels, or required checks currently reads or lints against this document.
It codifies a style contract that control-plane artifacts (issues, spec
packages, skills, tool output, PR bodies, vocabulary) should converge on
*incrementally* — existing docs, skills, and templates are not rewritten by
this PR to match it. See **Applying this doc** at the end for how adoption is
expected to happen.

**Controlling issue**: [#3807](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3807)
(design settled and plan-reviewed via the two comments this doc synthesizes:
["Control-plane guidance style"](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3807#issuecomment-4950452985)
and ["Operating-model completion"](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3807#issuecomment-4950524024)).
Canonical doctrine context: [#3949](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3949).

---

## The through-line

**Rich research, compiled decisions, narrow implementation, late publication.**

The issue is where uncertainty gets reduced — it is allowed to be expansive:
evidence, competing explanations, rejected approaches, external oracles,
corrected premises, review, rationale. A few thousand durable issue tokens are
cheap compared to the ~100k an agent burns rediscovering that context from
scratch. Once the issue converges, the repo **compiles** the settled result
into a compact form; the builder works from that compiled form, not the raw
history. Implementation stays narrow — the builder gets exactly the decisions,
invariants, and latitude it needs. Publication happens late — a PR is opened
once the work is effectively complete, not as a scratchpad.

Two more threads run through every artifact this doc governs:

- **Precise about what/why failed, generous about how to proceed.** A block or
  a failure names the exact artifact and boundary that didn't hold, then gives
  enough reasoning for a competent agent to repair or reinterpret it — not
  just enough to silence the check.
- **Positive path first.** Guidance leads with the thing to do ("create or
  reconcile the controlling issue, compile the reviewed spec package, then
  begin via `/start-work`"), not a list of things not to do. Prohibitions are
  reserved for genuine hazards (see §6).

---

## 1. Block at the earliest reliable boundary

A late block is usually a design failure — the cost of the mistake it catches
already happened before the check ran. Each class of defect has an earliest
point where it is reliably detectable; that is where the block belongs, not
one step later where it's merely *convenient* to check.

| Defect class | Earliest reliable boundary |
|---|---|
| Missing or unreviewed issue context | Before a writer worktree is created |
| Malformed or incomplete staged artifact | Before commit |
| Missing behavioral proof | Before push / publication |
| Stale review or failed remote integration | Before merge |

This is the same boundary `/start-work` already enforces pre-mutation (see
[`.claude/commands/start-work.md`](../../.claude/commands/start-work.md) and
[#3971](https://github.com/EffortlessMetrics/perl-lsp-swarm/issues/3971), the
issue that introduced it) and the shape the commit-gate feedback ladder
(#3786) and RIPR pre-push output are converging toward — this doc names the
general principle those concrete mechanisms instantiate.

---

## 2. Issue = working room, spec package = compiled builder interface

The GitHub **issue is the working room**: it may be as detailed as the problem
needs — initial evidence, competing explanations, architectural options,
rejected approaches, external oracles, corrected premises, reproductions,
review threads, uncertainty, rationale. Don't optimize issue prose for minimum
tokens; agents may add subsections as understanding grows.

Once the issue converges, the repo compiles it into linked, typed artifacts —
not a new monolithic format layered on top of what already exists. This repo
already treats proposals, specs, ADRs, plans, proof, support claims, and
closeouts as separate linked truth; the compiled view extends that model
rather than replacing it:

```
Issue #1234
  ├─ implements   → SPEC-LSP-HOVER-004
  ├─ governed-by  → ADR-ANALYSIS-OWNERSHIP-002
  ├─ depends-on   → SPEC-PARSED-SNAPSHOT-001
  ├─ proves       → INV-FRESHNESS-007 / INV-ANALYZER-REUSE-003
  └─ verified-by  → PROOF-HOVER-RANGED-EDIT-005
```

The **builder-facing projection is generated from this graph** — outcome,
scope, non-goals, settled decisions, invariants, proof obligations, builder
latitude, return-to-issue conditions — not hand-maintained as a second source
of truth. The issue holds history; the spec graph holds the repository
contract; the projection is what the builder actually reads.

**Reconcile, don't fork.** In this repo today that compiled projection *is*
spec-planner's `.spec/<issue#>-<slug>/{checklist.md, acceptance.md,
context.md}` packet (see [SPEC_TEMPLATE.md](SPEC_TEMPLATE.md)). The linked
ADR/SPEC/INV/PROOF graph above is the direction that packet should evolve
toward as the surrounding tooling (#3786, the ADR/spec network) matures — it
is not a second packet format to stand up in parallel. Where the two already
overlap (e.g. `acceptance.md` §Contracts already links to
[PARSER_CONTRACTS.md](PARSER_CONTRACTS.md)), treat that as the graph already
partially in place.

A builder should not need to reread fifty comments. It receives a packet that
already absorbed them.

---

## 3. Skills recommend a procedure, not dictate a ritual

A skill (`.claude/commands/*.md`) names a moment and a good default path
through it — it does not lock in a fixed model, a fixed agent count, a rigid
Haiku→Sonnet sequence, one-persona-per-stage, or a mandated private-reasoning
step. Contain:

- **When** to use it
- **Desired outcome**
- **Recommended procedure**
- **Options to consider**
- **Stop / return conditions**
- **Durable artifacts** it produces
- **1–2 worked examples**

`/start-work` already follows this shape: it names the pre-mutation moment,
states the outcome (a settled packet before a worktree exists), gives a
procedure with numbered steps, and names explicit stop conditions — without
prescribing which agent does the handoff work downstream.

---

## 4. Tool output: compiler shape, coach direction

Every piece of mechanical tool/gate output (staged checks, RIPR gap output,
CI receipts, lint results) follows the same shape:

**result · why it matters · affected artifacts · fix · rerun · what remains**

- **One fix when the repair is mechanical.** If there is exactly one correct
  next action, state it directly — don't force a human or agent to choose
  among equivalent options that don't exist.
- **Real options when judgment is required.** If the repair depends on intent
  the tool can't infer (is this a real regression or an intentional behavior
  change?), present the actual choices, not a false single answer.
- **Targeted rerun**, not "rerun everything" — name the specific command that
  re-checks just the thing that failed.
- **What remains required later** — a warning that something is deferred, not
  waived (e.g. "static RIPR checked; mutation proof still required before this
  merges given the weak test oracle").

**Tone**: neutral-warm. Terse on success — one line is enough
(`✓ staged checks passed — 8 selected, 3 cached, 1.8s`). Explanatory and
actionable on failure — enough reasoning that a competent agent can decide the
repair, not just enough to name the failure. No celebration, no scolding.

**The test for "is this rationale worth including"**: *does it change how a
competent agent would repair or interpret the result?* If yes, include it. If
it's decorative, cut it.

---

## 5. Workflow vocabulary

A small, fixed vocabulary for how blocked/uncertain states get communicated,
so redirection never reads like an emergency when it isn't one:

| Term | Meaning |
|---|---|
| **ADVISORY** | A grounded concern is flagged; work may continue. Not a block. |
| **CLASSIFICATION REQUIRED** | The repository needs an explicit decision before the next transition (e.g., is this changelog entry a product-facing fragment or an exemption?) — blocking here prevents the decision from being lost to release-time archaeology. |
| **BLOCKED** | A deterministic artifact or proof defect — an objectively invalid state, not a judgment call. |
| **RETURN TO ISSUE** | Implementation invalidated a planning premise (root cause, ownership/authority, cross-lane scope, acceptance criteria, proof seam, or risk/rollback changed). A return is useful evidence that the plan needs updating, not a failure — repeated returns on the same class of premise mean the *synthesis* needs improving, not the builder. |
| **NOT PROVEN** | The expected verification instrument didn't run, or produced no usable evidence — distinct from a failing proof. Warn and direct; don't silently treat absence of proof as proof of correctness. |
| **STOP** | Safety, ownership-collision, destructive, or irreversible risk. The one posture that halts unconditionally regardless of confidence. |

Proof **obligations** are durable contracts (e.g. "a hover-ranged-edit change
needs a freshness invariant proof"); the **suggested commands** that discharge
them are today's efficient implementation of that contract, not the contract
itself — except where the exact instrument *is* the contract (e.g. a
release/conformance harness whose specific invocation is normative).

---

## 6. Positive path first, prohibitions for genuine hazards

Lead every piece of guidance with the thing to do. Reserve explicit "never"
language for hazards where the cost of getting it wrong is high and the
correct alternative is unambiguous:

- Never use `--admin` to bypass a red required gate (the sole documented
  exception is the narrow, criteria-bound verified-treadmill-break operator
  path in
  [serialize-merges-and-cancellation.md](../concepts/serialize-merges-and-cancellation.md#current-repo-state-merge-queue-available-strict-up-to-date-relaxed-2026-06-13) —
  an admin merge may proceed only when the PR is `deep-reviewed`, at least 2
  of 3 required checks are green on the current SHA, and the failing check is
  confirmed a measurement artifact; this is an operator escape hatch for a
  known artifact class, not a general license to override review).
- Never resolve a substantive P1 finding just to clear the ruleset.
- Never let two writers hold the same branch concurrently.
- Never publish secrets.

Everything else — including most process guidance — reads better and is
followed more reliably as "do X" than as a list of "don't do Y."

---

## 7. Publication, review, and evidence discipline

These mechanics all serve the same through-line (rich research → compiled
decision → narrow implementation → late publication):

- **PRs publish late.** A PR opens once implementation is coherent and
  cleaned, focused proof (red/green plus an opposite-direction guard) exists,
  static analysis (e.g. RIPR diff exposure) has been checked, and local review
  has happened — "ready," not a remote scratchpad for iterating in public.
  Draft PRs remain legitimate for concrete reasons: remote-only evidence
  needed (e.g. a CI-only platform), cross-session or cross-person
  collaboration, early visible ownership, or an unavailable local test
  platform — not as a default starting state.
- **Independent review is risk-gated, not ritual.** A separate review pass
  earns its cost when architecture/ownership, concurrency/lifecycle,
  parser/compiler semantics, security, `unsafe`, user-visible LSP behavior
  with real ambiguity or broad blast radius, CI/merge/release/control-plane
  authority, public API/compat, or a weak/disputed test oracle is in play.
  Same-owner review is sufficient for a small well-oracled bug fix, narrow
  test-strengthening, a mechanical compat update, a low-risk refactor with
  unchanged behavior and strong proof, or a routine dependency refresh. Add a
  review pass because it improves falsification, not because the pipeline
  demands a persona for its own sake.
- **Reviewed plans expire after one day.** Before building from a plan more
  than 24h old, a cheap refresh checks current main, open PR/branch
  collisions, affected symbols/paths, dependency drift, the proof seam, and
  external assumptions. A clean refresh is a short report ("basis abc→def, no
  material changes, spec remains executable"). Only re-run the full plan
  review if the refresh finds something material changed.
- **Review epochs, not a flat re-review on every push.** Private
  implementation → local review → repair → publish ready → one broad
  bot+human pass → batch the accepted fixes → focused re-review of only the
  *changed* seams → final validation → merge. A substantive push starts a new
  epoch but re-runs only the dimensions that push actually invalidated, not
  every dimension from scratch. Broad automated review tools start once,
  against a nearly-finished head — not on every intermediate push.
- **Evidence tiers**, matched to how long the evidence needs to survive:
  - **Cache-only** — formatter checks, schema parsing, conflict scans, lint:
    cheap, rerun trivially, no need to persist.
  - **PR-lifetime** — staged-tree contract checks, RIPR exposure, the focused
    tests selected/run, affected-package routing, spec-graph validation,
    Changie disposition, review dispositions: persist for the life of the PR.
  - **Long-lived** — mutation/revert proof, external Perl/LSP oracle
    verification, performance receipts, concurrency/lifecycle proof, support-
    tier claims, ADRs, release/audit records: persist independent of any one
    PR, typed and linked, bound to spec + head SHA.
  - **Rule of thumb**: persist evidence when another session, reviewer, or
    release needs to rely on it without re-running the check itself.
- **Context by role**, not one-size-fits-all: the builder gets the compiled
  spec projection plus invariants, proof obligations, scope/non-goals,
  latitude, return-conditions, and evidence links — not the full issue thread
  by default. The
  plan reviewer gets the full issue thread, research, rejected options, the
  compiled spec, and oracle evidence. The implementation reviewer gets the
  compiled spec, the diff, invariants, proof results, and material
  deviations. Anyone can fetch more when the compiled view is insufficient —
  this bounds default context cost, it does not hide information.
- **Builder latitude is explicit**, so builders neither reinvent from a vague
  plan nor treat an over-specified plan as a brittle recipe to follow
  literally:
  - **May** (no return needed): private helper names, internal decomposition,
    relocating a focused test within the owning crate, a narrower
    implementation that still preserves every invariant, or a stronger proof
    than required.
  - **Return to the issue** when root cause, ownership/authority, cross-crate
    or cross-lane scope, acceptance criteria, the proof seam, or risk/rollback
    materially changes from what the plan assumed.
- **Fast path is about preserved decisions, not change size.** The test is:
  *does this change contain a decision worth preserving?* — not "is the diff
  small." No-issue-normally: typo/formatting fixes, deterministic
  regeneration with no generator change, a lockfile refresh with no source
  change, a mechanical version sync, or a private rename with no behavior
  change. Short-or-existing-issue: a test-only change that defines new
  behavior, a one-line behavioral fix, a dependency update carrying compat
  code, a dependency update that closes a defect, a docs change that alters a
  support contract, a CI change altering what's required, or generated output
  from a generator change. Issue-first is not a role relay — the same agent
  may create, research, compile, implement, and publish ordinary work; the
  issue exists to preserve intent, not to force a handoff between agents.

---

## 8. Preferred vocabulary

**Prefer** (plain, general-purpose terms): issue · plan · spec package ·
current head · scope · non-goals · invariant · evidence · proof · owner ·
writer · ready to build · merge-ready · reconcile · fast path.

**Use carefully** — only when naming a real, specific system property, not as
a synonym for "process" in general:

- **control plane** — the machinery that actually selects, routes, protects,
  and reconciles work (CI gates, required checks, the reconciler) — not a
  fancy name for "how we do things."
- **gate** — a coarse pipeline stage with a real entry/exit condition (see
  [PIPELINE_GATES.md](PIPELINE_GATES.md)) — not any arbitrary checkpoint.
- **receipt** — an actual structured, persistent result bound to a SHA — not
  every log line or narrative status update (see *Receipts + PR cockpit* in
  [modern-claude-operating-model.md](../swarm/modern-claude-operating-model.md#receipts--pr-cockpit)).
- **builder-ready** — the specific label/live-signal state this repo already
  treats as authoritative (see
  [LIVE_SIGNALS_VS_LABELS.md](LIVE_SIGNALS_VS_LABELS.md)) — not "seems done."
- **lifecycle** — a defined sequence of states with real transitions, not a
  loose synonym for "workflow."

**Normative language**: ordinary English for human-facing prose ("Create the
issue before taking write ownership"). Reserve formal MUST/SHOULD/MAY language
for machine contracts and schemas, where:
- **must** = safety, ownership, or irreversibility is at stake
- **should** = the paved-road default, deviate with reason
- **may** = legitimate judgment call, either choice is fine
- **normally** = a strong default that has real, nameable exceptions
- **never** = sparingly, reserved for genuine hazards (see §6)

**Bypass** is not a normal workflow feature. It exists for narrow, named
cases — a WIP checkpoint, a confirmed tool defect with a filed issue, an
unrelated environment failure, or emergency preservation of work — and it
never satisfies later publication proof on its own. A bypass always records
the reason and what remains required before the artifact can be considered
complete.

### Note: "claim boundary" → "what this establishes" (deferred, not applied here)

This doc's source discussion proposes replacing **"claim boundary"** with
**"what this establishes"** / **"proof scope"** / **"supported conclusion"** /
**"not established by this change"** — the term reads more like an assertion
of *what got proven* and less like a legal-sounding limitation. That rename is
**not applied in this PR**. `CLAUDE.md` (§Publication and proof) and
`.github/PULL_REQUEST_TEMPLATE.md` currently use "Claim Boundary" as the
section name, and changing it is a coordinated rename across both of those
plus the operating-model doc — not something a single reference doc should
silently redefine out from under existing artifacts. Treat "what this
establishes" as the **preferred term for new prose that isn't bound to the
existing template's literal section heading**, and treat the rename itself as
a tracked follow-up, not a fait accompli.

---

## Applying this doc

This is a reference, not a rule — nothing here is mechanically enforced today.
Adoption is expected to be incremental and reconciliation-first, land as
targeted slices, and never retroactively rewrite artifacts already in flight:

- Issue and research-comment templates move toward the roomier structure in
  §2 over time.
- The spec-planner `.spec/` packet evolves toward the linked ADR/SPEC/INV/
  PROOF graph in §2, rather than a second format appearing alongside it.
  New skill descriptions and skill updates adopt the shape in §3 and the
  latitude/return-condition language in §7.
- Staged-check and RIPR tool output move toward the shape in §4 and the
  vocabulary in §5 as those systems (#3786 and related work) evolve.
- The late-publication flow, review epochs, and 1-day plan refresh in §7
  apply to the operating-model doc and the review/merge lanes as those are
  revisited.

None of the above is committed to a schedule by this PR — this doc exists so
that when that work happens, it has one settled style to converge on instead
of re-deriving it per-PR.

---

## See also

- [docs/concepts/external-truth-gate.md](../concepts/external-truth-gate.md) —
  the internal-vs-external-truth distinction this doc's evidence-tier
  language assumes.
- [docs/reference/ORCHESTRATION_DOCTRINE.md](ORCHESTRATION_DOCTRINE.md) — the
  gate-model mentality and direction this doc's tool-output and vocabulary
  guidance operates inside.
- [docs/swarm/modern-claude-operating-model.md](../swarm/modern-claude-operating-model.md) —
  truth hierarchy, delegation model, and the receipts/PR-cockpit shape this
  doc's publication guidance (§7) refines.
- [docs/reference/PIPELINE_GATES.md](PIPELINE_GATES.md) — the gate structure
  this doc's "block at the earliest reliable boundary" principle (§1) maps
  onto.
- [docs/reference/SPEC_TEMPLATE.md](SPEC_TEMPLATE.md) — the current concrete
  form of the "compiled spec package" this doc describes in §2.
- [.claude/commands/start-work.md](../../.claude/commands/start-work.md) — the
  existing pre-mutation guard that already applies §1's earliest-boundary
  principle and the skill shape in §3.
