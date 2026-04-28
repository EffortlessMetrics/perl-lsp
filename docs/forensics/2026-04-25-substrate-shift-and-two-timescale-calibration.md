# Substrate Shift + Two-Timescale Calibration

**Window**: 2026-04-25 synthesis from session conversation, capturing 2026-04-24/25 substrate transition
**Audience**: orchestrator, anyone reading older calibration data, anyone tempted to over-tune verification heuristics
**Purpose**: document the literal-yesterday substrate shift (Codex 5.4 → 5.5 + ChatGPT-Pro upstream research) and the meta-principle it illustrates — two unrelated timescales both argue against perfectionism in calibration, so "decent and ship" beats "fine-tuned and stale."

---

## The empirical observation: the substrate shifted under us

In the 24-48 hours preceding 2026-04-25, two things changed in the methodology's upstream layer:

1. **Codex 5.5 launched** (Thursday 2026-04-24, per operator). Replaced 5.4 as the dominant upstream PR-generation model. Subjective quality and hallucination-rate appears improved versus 5.4, though formal calibration hasn't been done yet.
2. **ChatGPT-Pro + GitHub repo connector entered the upstream-research loop**. Added a structured pre-coding research step that pulls relevant context from the actual repo before Codex generates PRs. Reduces the "scope drift" and "wrong direction" failure classes at their source.

These are both **upstream** changes. Neither modifies the verification ladder, agent definitions, or any in-repo infrastructure. But both should change downstream behavior in measurable ways:
- Lower hallucination rate in Codex bursts → fewer ladder catches per PR
- Better upstream direction → fewer "wrong scope" verdicts at maintainer-issue review
- Higher base quality → deep-review's residual catch surface narrows
- Some risk-classes that previously needed downstream verification now don't reach the ladder at all

**The unverified part**: I don't have post-shift calibration numbers. The 6.3% scout-error figure in `feedback_research_verifier_roi.md` is a 5.4-era number against the old upstream loop. The `feedback_codex_ensemble_pattern.md` ratios are 5.4-era. The deep-review near-100% catch rate is 5.4-era. None of these have been re-measured against 5.5 + ChatGPT-Pro yet.

This matters because **prior calibration is now wrong-by-default** — not because it was wrong, but because the conditions it described changed underneath. Acting on stale calibration is a form of confidently-wrong reasoning.

---

## The meta-principle: two timescales, same conclusion

The substrate shift exposes a meta-principle that applies regardless of any specific shift. There are two timescales at which methodology calibrations get stale, and both argue against perfectionism:

### Macro timescale: pre-AI vs. now

The methodology exists at all because the cost floor for high-quality software development dropped 30-100× compared to traditional ($5-10k/PR senior engineer attention) baselines. Optimization beyond "decent" was unaffordable in the pre-AI baseline; now it's just unnecessary.

Every "decent" outcome here is unimaginably better than the alternative the macro baseline offered. Tuning verification heuristics to capture the last 5% of quality past "decent" is optimizing the edge of an already-better-than-feasible regime.

### Micro timescale: literal yesterday's substrate vs. today's

Within the AI era, the substrate moves fast enough that any calibration sharper than "decent" gets stale before it pays back. Codex 5.5 just invalidated whatever fine-tuned heuristics were calibrated against 5.4-burst-quality. Some future version (5.6, 6.0, whatever) will invalidate today's calibrations against 5.5.

The cycle time is short enough — measured in days, not months — that optimizing the third decimal place of any calibration metric is wasted effort. By the time the optimization is verified, the substrate has shifted enough to invalidate it.

### Both arguments converge on the same operational rule

Macro says: "decent is unimaginably better than the alternative."
Micro says: "anything sharper than decent is overfitting to a substrate that's about to change."

Independent reasons. Same conclusion. **Decent and ship beats fine-tuned and stale, on both timescales.**

---

## Operational consequences

### Don't tune verification heuristics past "decent"

The thresholds in the verification ladder (when does deep-review fix-forward vs. bounce, when does diff-audit flag scope drift, when does maintainer-PR escalate, etc.) should be **decent and stable**, not finely tuned. Tuning them harder than the substrate is moving is overfitting.

This applies to:
- Model temperature and prompt micro-engineering
- Decision thresholds in agent definitions ("if N >= 3 PRs match the cascade pattern...")
- Calibration ratios in operator playbooks ("DEFER vs CLOSE at X% confidence")

A "decent" version that holds across substrate shifts is more valuable than a "perfect" version that needs re-tuning every week.

### Treat older calibration numbers as bound to the substrate they were measured against

The 6.3% scout-error figure, the ensemble closure ratios, the "near-100% deep-review catch rate" — all are bound to `(model_version, upstream_loop, downstream_diversity)` as of the date measured. When any of those three change, the number needs re-verification, not just re-quoted.

Concretely: when reading any forensics doc or memory entry that contains a percentage or ratio, look for the substrate stamp. If absent or older than the current substrate, treat the number as a hypothesis to verify, not a fact to act on.

### Schedule recalibration after substrate shifts

When the upstream loop changes (model version, upstream-research source, plan family activated), schedule a recalibration pass within 1-2 sessions. The memory-recalibrator agent class (added 2026-04-25) is the executor; the operator's job is to recognize when a substrate shift has happened and trigger the recalibration.

Substrate-shift signals:
- Operator notes "Codex X.Y launched" or "we're now using ChatGPT-Pro for upstream research"
- A new model family enters the downstream layer (GLM, Fireworks, Minimax, OpenCode)
- A new agent class is added that absorbs a previously-residual catch class
- Throughput metrics shift markedly without explanation

Each of these warrants re-verifying time-sensitive calibrations.

### Forensics and memory entries should be authored with the half-life in mind

Per the authoring conventions doc: date stamps in filenames, substrate-version named when load-bearing, calibration numbers tagged with the substrate they were measured against. This lets the recalibrator (and any reader) tell at a glance which fragments need re-verification when the substrate moves.

The corollary: a forensics doc written without substrate stamps is harder to maintain. It's not a fatal flaw, but it costs more recalibration work to keep useful.

---

## What the 2026-04-25 substrate shift specifically implies

Concrete, actionable expectations for the next few sessions:

1. **The verification ladder will appear to "thin"** — fewer catches per PR — without process changes. This is not degradation; it's the upstream catching more risk-classes before they reach the ladder. Resist the urge to interpret thinning as ladder weakness.

2. **The 5.4-era catch ratios in existing forensics will look "off"** — measured ratios won't match what the docs claim. Recalibrate; don't assume the docs are still right.

3. **Some agent classes added against 5.4 may have less work to do** — research-verifier, in particular, is calibrated against a 6.3% scout-error rate that may be lower with 5.5 + ChatGPT-Pro upstream research. Re-measure before deciding to retire (per the principled retirement criterion in the defense-in-depth doc).

4. **The three-way-match agent (added 2026-04-25) is calibrated against 5.4-era red-TDD failure rates** (G1a 3 fixes, G1b 6 fixes per wave). 5.5 + upstream research likely reduces these. The agent still earns its keep at lower rates, but its marginal catch rate should be tracked over the next few waves.

5. **The "expensive deep-review catches" of the past 2-3 sessions should be cross-referenced** to see which would have been caught upstream by 5.5 + ChatGPT-Pro. That's the empirical evidence for which risk-classes the substrate shift moved.

These are hypotheses, not predictions. The recalibration pass turns them into measurements.

---

## What this isn't

- **Not "the substrate change makes the ladder unnecessary."** Defense in depth still applies; the ladder's residual catch surface remains valuable. The shift changes which risk-classes are residual, not whether residual catching is needed.
- **Not "calibrations don't matter."** They do — but they should be **decent and held loosely**, not fine-tuned and held tightly. The cost of being wrong is bounded; the cost of constant re-tuning is unbounded.
- **Not "we should pause the methodology to recalibrate."** The methodology runs continuously; recalibration is a periodic background task, not a phase. The recalibrator agent runs on a schedule, not a freeze.
- **Not "the substrate will keep getting better."** Some shifts will degrade quality (model regressions, prompt drift, training cutoff effects). Half-life-aware fragments + recalibration are the structural defenses against degradation just as much as against improvement.

---

## The deeper observation

Both timescales are saying the same thing for unrelated reasons that happen to align. That's significant. When two independent arguments converge on a single operational rule, the rule is more robust than either argument alone.

The macro argument depends on AI being meaningfully cheaper than traditional development (true today, may not be true if pricing changes radically).
The micro argument depends on substrate moving fast (true today, may slow if model release cadence stretches out).

If only one held, the rule "decent and ship" would be conditional on that argument's premise. Because both hold independently, the rule is robust to either premise weakening — as long as either is true, "decent and ship" is right.

This is the same property defense-in-depth gives the verification ladder: redundancy across uncorrelated dimensions makes the conclusion robust to single-dimension failure. Same architectural principle, applied at the methodology-rule layer instead of the verification-layer.

---

## Related forensics + memory entries

- `2026-04-25-defense-in-depth-verification-architecture.md` — the ladder structure that absorbs substrate shifts
- `2026-04-25-verification-economics-push-upstream.md` — what "thinning" means and why it's the right direction
- `2026-04-25-forensics-and-memory-authoring-conventions.md` — half-life dating and substrate-version stamps
- `2026-04-25-forensics-as-prompt-fragments-architecture.md` — why the recalibrator agent exists in the architecture
- `feedback_research_verifier_roi.md` — 5.4-era calibration that needs re-verification against 5.5
- `feedback_codex_ensemble_pattern.md` — 5.4-era ensemble ratios that need re-verification
- `feedback_deep_review_bug_catch_roi.md` — 5.4-era deep-review catch rate that needs re-verification

---

## Applies to

Reference this doc when:
- Reading any forensics doc or memory entry containing a percentage or ratio
- Tempted to fine-tune a verification heuristic past "decent"
- Noticing the verification ladder catching fewer bugs per PR than docs predict
- Onboarding a new operator who needs to know why "decent and ship" is the rule
- Triggering a memory-recalibrator pass after a substrate shift
- Evaluating whether to retire a layer (substrate shift may have moved its catch class upstream — verify before retiring)
