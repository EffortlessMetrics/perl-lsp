# Stochastic-Ready Pipelines

## The posture

Every artifact emitted by a pipeline — agent summary, CI status report, tool finding, test name, issue claim, PR body, doc update — is a **claim with a reliability profile**. Some claims are highly reliable (a compiler error on a known source file). Others drift routinely (a coverage percentage computed by a tool whose transformation logic was not tested). Most fall somewhere between.

A stochastic-ready pipeline does not assume artifact correctness. It treats every artifact as evidence, applies the appropriate confidence weight, and routes to independent verification when the stakes exceed the reliability of the primary signal.

This is calibration, not hostility. Humans make the same errors — overclaiming in PR summaries, misreading CI dashboards, writing tests that test the wrong thing. Agents produce the same drift faster, with more confident prose and at higher volume. The controls a stochastic pipeline needs are exactly the controls mature human organizations already apply: independent checks, sampling, receipts, escalation paths, and post-incident learning. The stochastic pipeline is the same engineering discipline at higher velocity.

---

## Artifacts that drift

The following artifacts have documented drift patterns in practice. Each is listed with its characteristic failure mode.

**Agent summaries**
An agent reads a diff, summarizes it, and posts a comment. The summary is generated from context — which may include stale prior agent comments, truncated tool output, or a model's training-data priors. The summary can claim a function was added when it was renamed, assert a test covers a path it does not reach, or omit a change that appeared in a second file the agent did not read. Agent summaries compound: an accuracy-scout reading a hallucinated issue summary may propagate the error rather than correct it.

**CI status**
CI green on a given PR does not mean CI green on the current HEAD SHA. A CI run started against an earlier commit can still be attached to a PR after the PR's branch has been updated. The check is technically passing — but it is passing for a different version of the code. This is the "stale green" failure: a required check that has been satisfied but whose satisfaction is no longer meaningful (see PR #1425 verify-the-instrument pattern). CI status must be read as "this SHA passed this check at this time," not "this PR is clean."

**Tool reports**
Tools compute an answer from inputs. If the inputs are wrong — wrong file paths, wrong exclusion filters, wrong base comparison, wrong version of the tool relative to the version CI uses — the output is wrong. Coverage tools that exclude the wrong line ranges, linters that use a different rule set than the one in the repo's config, and diff-auditors that diff against the wrong base are all instances of a tool producing a confident numeric result from corrupted inputs (see #1453 Codecov integration-counting, #1232).

**Test names and test pass/fail**
A test named `test_foo_returns_correct_result` passes. This is not evidence that `foo` returns the correct result — it is evidence that the test's assertions are satisfied. If the test was written against incorrect behavior (hazard class 5 from hazard-class-invariants.md: "test encodes the bug"), the test will pass precisely when the bug is present. Test pass/fail must be read as "these assertions hold," not "the behavior is correct."

**Issue claims**
An issue reports that function `parse_heredoc_body` crashes on empty input. By the time a builder reads it, the function may have been refactored into a different module, renamed, or the crash may already be fixed by an unrelated merge. Issue claims are point-in-time observations. They require mechanical fact-checking (does the named function exist at the named path? is the reported behavior still observable?) before they drive implementation.

**PR bodies**
A PR body is written by the builder or an agent after implementation. It may describe the intended change, not the actual diff. Scope drift — where the diff contains changes the PR body does not mention, or where the PR body claims changes that are not in the diff — is a routine finding at diff-audit time. PR bodies are an author's self-report; they must be cross-checked against the diff as primary evidence.

**Documentation**
Docs describe the system at the time they were written. As code moves, docs drift. A doc that says "the parser resolves this via `parse_indirect_object` in `crates/perl-parser/src/stmt.rs`" becomes wrong the next time that function is renamed or moved. Docs are high-reliability for stable contracts and low-reliability for concrete file/function pointers.

---

## The controls

The controls are not exotic. They are the ordinary engineering controls that human organizations developed for exactly this reason — that human workers also produce drift, also overclaim, also fail to cross-check.

**Independent checks**
The same claim is verified by an agent that did not produce it. The accuracy-scout does not read the builder's PR body to check correctness — it reads the actual files. The green-ci agent does not read the reviewer's comment that CI is passing — it reads the current SHA's check state directly. Independence is the mechanism that prevents one agent's error from propagating through the pipeline unchallenged.

**Sampling**
Not every artifact is independently verified on every pass — the cost would be prohibitive. Sampling identifies which artifacts to verify: anything that is being acted upon (a spec that will drive implementation, a CI status that will gate a merge, a test failure that will block progress), anything where drift has been observed before for that artifact class, and anything where the stakes of a wrong artifact are high (merge decisions, release claims).

**Receipts**
A receipt is a verifiable artifact attached to a claim: the SHA the CI run was computed against, the specific line numbers the coverage tool excluded, the exact output of the tool whose result is being reported, the timestamp and PR number of the "already fixed" claim. Receipts make claims falsifiable. An agent summary that says "CI is green" is not falsifiable. An agent summary that says "CI is green on SHA `e2be839e8` as of the run started at 14:32 UTC" is falsifiable and durable.

**Escalation paths**
When a pipeline artifact is in conflict with another, or when a claim cannot be verified through available evidence, there must be a path for raising the conflict rather than silently resolving it in favor of one claim. In this repo, the pr-responder, diff-auditor, and green-ci agents each represent an escalation layer for their specific artifact class. The presence of a `needs-ci-fix` or `needs-diff-fix` label is a structured escalation: the pipeline has detected a conflict and is routing it rather than suppressing it.

**Post-incident learning**
When an artifact produces a wrong routing decision — a stale green that allowed a broken merge, a test that encoded the bug and blocked the fix, an agent summary that claimed the wrong function was changed — the incident becomes a learning entry. The entry records which artifact class drifted, under what conditions, and what the detection mechanism was. This is the mechanism by which the pipeline's reliability profile improves: not by assuming correctness, but by logging and encoding the failures (see docs/learnings/).

---

## Calibration, not paralysis

Treating every artifact as evidence with a reliability profile does not mean treating every artifact as suspect in every context. That would be operationally useless.

The calibration question is: **what is the cost of acting on this artifact if it is wrong?** A low-stakes wrong artifact (a doc that describes an internal function incorrectly) warrants a low verification investment. A high-stakes wrong artifact (a CI status that gates a merge to main) warrants direct verification against the primary source.

The pipeline should encode its reliability expectations: which artifact classes are high-reliability by construction (a Rust compiler error), which are medium-reliability and benefit from sampling (agent summaries), and which are routinely wrong and require independent verification before acting (stale CI status, self-reported PR bodies for scope claims).

---

## Distinction from verify-the-instrument

The verify-the-instrument pattern (referenced in docs/writeups/2026-06-agentic-maintenance-field-notes.md and several learnings entries) is a specific tactic: when a measurement is suspicious, check whether the measuring tool itself is the source of the error. The ripr 0.5.0/0.9.0 version divergence (#1289, #1329) and the Codecov integration-counting bug (#1453) are instances: the artifact (coverage percentage, seam count) looked valid but the instrument was computing the wrong answer.

This document describes the broader posture that motivates and contextualizes that tactic. Verify-the-instrument is the answer to "why is this specific measurement wrong?" Stochastic-ready pipelines is the answer to "why do we check, and how do we build a system that catches this class of error systematically?"

The posture precedes the tactic. If the posture is not held — if artifacts are assumed correct until proven wrong — verify-the-instrument is never invoked, because there is no trigger for the check.

---

## Relation to other patterns

- **Slow stochastic compiler** (`slow-stochastic-compiler.md`) — the framing for the fleet as a whole: intent is compiled through stochastic stages into PRs. Each stage emits artifacts with reliability profiles; this doc describes how those artifacts should be consumed.
- **Shift-left ladder** (`shift-left-ladder.md`) — the cheapest place to catch a drifting artifact class is the earliest feasible layer; pipeline design should route artifact verification to the cheapest sufficiently reliable rung.
- **Hazard-class invariants** (`hazard-class-invariants.md`) — class 5 (test encodes the bug) and class 6 (coverage measurement integrity) are concrete instances of artifacts with known-bad reliability profiles; their controls follow directly from the stochastic-ready posture.
- **Model conformance** (`model-conformance.md`) — when two artifacts conflict (two agents report different behavior for the same function), the conformance discipline provides the resolution procedure: enumerate claims, check against observed behavior, find the outlier.
- **Human corrects substrate** (`human-corrects-substrate.md`) — the human operator's role is highest when artifacts from multiple pipeline layers are in conflict and no agent-level escalation path can resolve it; the human reads the primary sources the agents cannot access.

---

## Failure modes to watch

These are the recurring ways a stochastic pipeline degrades, with the controls that catch each.

**Agent overclaim** — symptoms: "red tests added" but they pass pre-fix; "CI green" but only advisory checks are green; "no suppression needed" before a live receipt; PR body says tests fail intentionally after they no longer do. Controls: valid-red proof; required-check truth (not advisory); PR-body-vs-diff review; raw-artifact capture.

**Substrate mismatch** — symptoms: branch-protection assumptions wrong; main moves unexpectedly; CI starvation; tool output schema changed; default `gh` limits hide items. Controls: substrate-model docs; preflight model-conformance check; heartbeat state packets; an explicit merge-queue / strict-up-to-date decision.

**Doctrine bloat** — symptoms: docs expand faster than enforcement; agents cite docs but do not follow them; specs become stale inventory. Controls: mechanical spec validators; learning entries linked to specs/tests; a periodic stale-doc scout.

**Merge thrash** — symptoms: PRs green 2/3 forever; update-branch cancels in-progress CI; consolidated watch churns too many PRs. Controls: parallel builds, paced merges; never update mid-CI; small batches; an explicit merge-queue policy decision.
