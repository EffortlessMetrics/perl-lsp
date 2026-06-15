# Doctrine Is a Hypothesis Until Enforced and Measured

*Portable concept. Grounded in perl-lsp 2026-06 session. See also: [verify-the-instrument](verify-the-instrument.md), [slow-stochastic-compiler](slow-stochastic-compiler.md), [hazard-class-invariants](hazard-class-invariants.md), [human-corrects-substrate](human-corrects-substrate.md).*

---

## The thesis

Doctrine that has been minted to explain past incidents feels like validated truth. It **predicted** those incidents in hindsight, which is satisfying. But explaining-in-retrospect is not the same as predicting-forward. A framework that explains everything that already happened risks explaining nothing that will happen next.

By the discipline of [verify-the-instrument](verify-the-instrument.md) — self-report is unreliable until cross-checked against ground truth — your freshly-minted doctrine is itself a self-report. You believe it because it fits the historical incidents you selected to validate it. Until the **next cycle** of work, you have no empirical proof it will prevent the class it claims to prevent.

The 2026-06 session generated confident doctrine that repeatedly **predicted its own second-half incidents**. That pattern — "we encode this rule, then we observe it being violated, then we encode a deeper rule" — is suggestive but not conclusive. It may indicate prescient rule-making. It may indicate that you are encoding the wrong layer and will discover deeper rules forever.

---

## The bar for validation

A doctrine is not validated until **both** of these hold:

1. **It is enforced.** Not prose, but named mechanism. Either:
   - Automatic (a new CI check, a type invariant, a lint rule, a spec checklist row)
   - Procedural (a maintainer-doctrine check, a review gate, a state machine that makes violation impossible)
   - See [enforcement-over-doctrine](enforcement-over-doctrine.md) for the full framework.

2. **The next cycle shows measurably lower incident-rate for that class.** You count incidents belonging to this class in the next campaign. The rate drops after enforcement is added. If it does not drop — if violations keep occurring despite the encoding — the doctrine **failed** regardless of how good it reads.

Until both conditions hold, the doctrine is a hypothesis, not a law. Running the system and observing it working is empirical validation. Explaining why a past failure makes sense is literary validation. Do not conflate them.

---

## Measurement hook: How to know

For the hazard classes encoded in 2026-06, track incident-rate-per-class across the next campaign:

| Class | Where to count |
|-------|---|
| Invalid red / gate-dishonesty | `docs/learnings/` entries tagged `gate-dishonesty`; CI incident log |
| Non-exhaustive silent drop | Parser contract test failures; `docs/learnings/` entries tagged `silent-drop` |
| Merged before review / gate-honesty | `docs/learnings/` entries tagged `mislabel`; PR audit trail (label timestamp vs. merge time) |
| Substrate break on merge | `docs/learnings/` entries tagged `substrate-break`; master CI breaks traced to instrumentation change |
| Master red after merge | CI log; release-notes blockers |
| Agent claim vs. ground truth | `docs/learnings/` entries tagged `claim-miss`; orchestrator routing errors |

If these counts do **not** trend lower after enforcement is in place (across at least two subsequent full campaigns), the hypothesis has failed. Encode why in a learning entry: "doctrine X was enforced but did not reduce incidents in class Y; reason:"  Then design a deeper or alternative doctrine.

---

## Corollary: Know when reflection is done

Synthesis is powerful up to a point. Past that point, more reflection is itself the failure mode: doctrine-bloat, busy-≠-valuable, generating sophisticated post-hoc frameworks that do not change behavior.

The markers that the reflection well is dry:

- You are explaining the same incident from three different angles and each explanation is academically correct but none adds new enforcement rules.
- You have written the doctrine but your next action is still "hope the team remembers" rather than "the system will not allow this now."
- The pattern only appears once in the historical record, and you are generalizing to future cycles without evidence it will recur.

When you hit those walls, stop reflecting. Start empirical: run the system, measure, observe where it breaks despite your shiny new rules, and file learning entries. The next refinement must be rooted in what actually happened, not in deeper analysis of what happened last time.

---

## Open question: The unverified orchestrator layer

There is deep-review for builders' code. There is green-ci for merge timing. There is diff-auditor for coherence. But there is no automated check on the **orchestrator's routing decisions themselves** — "is this the right PR to route to review-deep, or is it actually a spec problem?" — until the human operator catches the misroute and corrects it.

In 2026-06, the human was repeatedly that check: catching confidently-wrong calls where an agent (or the orchestrator) routed work to the wrong gate. The question remains: **can an orchestrator-decision check be added structurally** (more ground-truth verification before routing, a validator that asks "does the evidence actually support routing X?"), **or is the zoom-out judgment "is this the right loop?" irreducibly human**?

This question should be resolved empirically, not decided now:

1. Instrument the next campaign to track orchestrator misroutes (human corrections to agent routing calls).
2. Categorize them: spec gaps, builder scope drift, agent claim vs. ground truth, substrate instrument failure, or "human has context the log does not."
3. Design a check that would catch category 1–4 automatically.
4. Deploy and measure: does the check reduce orchestrator misroutes without creating false positives?

If it works, encode it as a structural layer. If false positives dominate, the judgment remains human-calibrated (operator learns when to override the check). Either way, you have evidence, not a guess.

---

## Related patterns

- **[enforcement-over-doctrine](enforcement-over-doctrine.md)** — Doctrine without enforcement is prose. This document assumes enforcement is in place; enforcement-over-doctrine specifies what "in place" means (named mechanism, auditable, testable).
- **[verify-the-instrument](verify-the-instrument.md)** — Your measurement of "did the incident rate drop?" is itself an instrument. Validate the counting methodology before trusting the result.
- **[slow-stochastic-compiler](slow-stochastic-compiler.md)** — Doctrine lives in the compiler-analogy as "compiler flags" (branch policy, merge economics, risk appetite) and "buggy pass" (orchestrator misroute). The frame includes the question of what layer catches it.
- **[human-corrects-substrate](human-corrects-substrate.md)** — When doctrine is wrong, the operator does not re-run the last agent. The operator corrects the substrate (the policy, the rule, the model) and subsequent passes work against corrected assumptions.

---

## Summary

| Stage | Status | What it means |
|---|---|---|
| Doctrine written, no enforcement | Hypothesis, credible | Could work, is not yet working |
| Doctrine written + enforcement added | Hypothesis, with a test | System will not allow violations; count the incidents |
| Next cycle: incident rate drops | Validated | Keep the enforcement; it works |
| Next cycle: incident rate unchanged | Falsified | The doctrine did not prevent what it claimed; analyze and revise |
| Reflection without empirical measure | Not actionable | Interesting idea, waiting for a run to validate it |

The worst state is doctrine with no enforcement and no measurement plan. That creates confident-sounding rules that explain everything and constrain nothing. Avoid it by making the progression explicit: write it, enforce it, measure it, iterate.
