# Downstream Catches Upstream: An Inverted Verification Pattern

**Date**: 2026-04-11
**Cross-references**: wisdom retrospective #4117, swarm-ops retrospective #4125,
layered verification protocol #4111, PR #4090 case study #4127

---

## TL;DR

The usual story of layered verification is that **upstream catches downstream**.
Plan-reviewers catch scouts. Reviewers catch builders. Deep reviewers catch
reviewers. Ops catches CI regressions. Each layer is upstream of the next and
trusts the downstream layers to be checked from above.

The 2026-04-11 perl-lsp session produced four incidents where the direction of
catch was *reversed*. A builder caught a plan-reviewer's wrong CLI flag name. A
docs agent caught the orchestrator's wrong GitHub org URL in its own prompt.
Another builder caught a stale revert instruction referring to a PR that was
never merged. A docs sweep caught a PR-number mistake the orchestrator had
already propagated into multiple agent prompts. In every case, a layer assumed
to be trustworthy shipped a plausible-but-wrong claim, and the layer that was
structurally *downstream* of it noticed — not because it was auditing upstream,
but because it had to actually execute the claim to do its own job.

This article names that pattern, walks the four incidents, and records three
classes of upstream error that downstream layers reliably catch.

---

## Why This Deserves a Name

The layered verification model is built on a trust gradient. Cheap scouts file
roughly-right specs. Accuracy-scouts verify mechanical facts. Plan-reviewers
stress-test the approach. Builders implement. Reviewers polish. Deep reviewers
check for correctness nobody else would catch. Each step is a filter for
mistakes introduced at or before its own level.

This framing is correct, but it's incomplete. It hides a second axis of catch
that the 2026-04-11 incidents made visible: **downstream layers routinely catch
errors that originated upstream, whenever the downstream layer has to execute
something the upstream layer only reasoned about**.

Upstream layers often describe tools, APIs, or repository state in the
*abstract*. They read documentation, they recall past facts, they read code,
they compose instructions. Downstream layers, by contrast, have to *do*
something with those claims — run a binary, dereference a URL, rebase onto a
commit that either exists or doesn't. The act of execution is itself a
verification step, and it catches exactly the errors that abstract reasoning
missed.

Four incidents in a single ~6-hour session is not enough data to formalize
this, but it is enough to name it.

---

## Incident 1: Builder catches plan-reviewer's wrong `--json` flag claim

**Chain**:

1. **Scout a6116a22** (issue #4070, engineering-health scorecard) proposed
   per-crate mutation scores as one of the headline metrics. The scout had not
   independently verified the `cargo mutants` CLI surface.
2. **Accuracy-scout aa50278a** verified that `cargo mutants --json --output
   <dir>` produces per-crate data. The flag exists. The accuracy-scout posted
   findings on #4070 and labeled the issue `accuracy-reviewed`.
3. **Plan-reviewer a9909643** stress-tested the combined spec and caught a
   missing piece: the existing nightly CI invocation
   (`.github/workflows/ci-nightly.yml`) ran `cargo mutants --timeout 60
   --no-shuffle` with no `--json`, so even if the xtask code parsed JSON, no
   JSON would exist to parse. The plan-reviewer added this as a load-bearing
   dependency in the spec: *"PR 1 must patch ci-nightly.yml alongside the
   `update_status.rs` changes — add `--json` to the cargo mutants
   invocation."*
4. **Orchestrator** relayed the note on the issue verbatim as a critical
   dependency for the builder.
5. **Builder a717174b** opened the work and, before editing the workflow,
   actually ran `cargo mutants --help`. The help output revealed that **the
   `--json` flag only applies to `--list` mode**. A full mutation run already
   writes `mutants.out/mutants.json` in the workspace root regardless of any
   flag. The missing piece wasn't `--json` — it was an `upload-artifact` step
   to preserve `mutants.out/mutants.json` after the nightly job ended. The
   builder scoped the fix accordingly and explained the correction in
   [PR #4124](https://github.com/EffortlessMetrics/perl-lsp/pull/4124).

**What lied**: The plan-reviewer's coordination note, and the orchestrator's
relay of it, both claimed the fix was "add `--json`." The accuracy-scout's
finding that `--json` exists as a flag was true but irrelevant — the flag
applies to a different subcommand mode than the one nightly runs.

**Why it lied**: Three layers above the builder operated on a semi-verified
claim about CLI semantics. The accuracy-scout verified that `--json` is a
valid flag (true). The plan-reviewer verified that CI was not currently
passing `--json` (true). Neither verified the specific invocation semantics —
whether `--json` applies to `run` mode or `list` mode. The unverified gap was
small enough that everyone stepped over it.

**Who caught it**: Only the builder, because only the builder had to actually
edit the workflow line that was supposedly wrong and ended up running the tool
to confirm it.

**The teachable fact**: When an upstream layer makes a semi-verified claim
about a tool's behavior, that claim can propagate all the way to implementation
without verification. The builder is the last line of defense for CLI
behavior claims that upstream layers made without running the binary
themselves.

---

## Incident 2: Docs agent catches orchestrator prompt error — wrong GitHub org URL

**Chain**:

1. **Orchestrator** wrote the swarm-ops article dispatch prompt
   ([PR #4125](https://github.com/EffortlessMetrics/perl-lsp/pull/4125)) with
   literal GitHub URLs referring to `stevedoyle/perl-lsp`. That org does not
   exist; the actual project lives at `EffortlessMetrics/perl-lsp`. The most
   likely explanation is copy-paste from stale memory or an older draft; no
   upstream layer existed to catch it.
2. **Docs agent a3004cfc** began writing the article. When adding
   cross-references, the agent noticed that the URLs in the prompt did not
   match any actual GitHub state (trivially verifiable via `gh repo view`).
   The agent fixed the references to the bare `#NNNN` style used elsewhere
   in `docs/articles/`.

**What lied**: The orchestrator's own prompt. Project metadata — org name,
repository URL, canonical link format — is supposed to be ground truth, and
orchestrator prompts are trusted by downstream agents as authoritative.

**Why it lied**: Orchestrator prompts have no upstream reviewer. Nothing sits
between the orchestrator writing a prompt and the agent executing it. A
factual error in the prompt will propagate unchecked until an agent happens to
notice that a claim doesn't match reality.

**Who caught it**: The docs agent, because writing cross-references meant
actually resolving the URLs to real GitHub state. An abstract reader would
have seen `github.com/stevedoyle/perl-lsp/pull/4090` as a plausible-looking
link and moved on.

**The teachable fact**: Orchestrator prompts can contain factual errors and
there is no layer upstream of the orchestrator to catch them. The agent
executing the prompt is the first and only check. This makes agent-level
verification *more* load-bearing for orchestrator-originated claims than for
claims that have already passed through the scout/accuracy/plan-review filter.

---

## Incident 3: Builder catches that a revert instruction had no upstream counterpart

**Chain**:

1. **Orchestrator** filed [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100)
   (pragma phase-block correction tracking issue) after the research-verifier
   caught the false-premise fix in PR #4090. The tracking issue instructed a
   builder to revert two things:
   (a) PR #4090's `NodeKind::PhaseBlock` arm in `PragmaTracker::build_ranges`
   (b) PR #4052's `walk_node` body-scan workaround in
       `crates/perl-lsp-diagnostics/src/lints/strict_warnings.rs`.
2. **Builder a29656fa** opened the work and ran `git log` to confirm the
   master state. PR #4090 was **closed unmerged** at 2026-04-11T11:04:55Z.
   Nothing from #4090 ever landed in master, so there was no match arm to
   revert. Only #4052's `walk_node` workaround needed reverting.
3. The builder scoped
   [PR #4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108)
   accordingly, described the corrected scope in the PR body ("PR #4090 was
   closed by the orchestrator before merge, so `PragmaTracker::build_ranges`
   has no `NodeKind::PhaseBlock` arm to revert there"), and shipped a minimal
   19-line revert plus inverted test assertions.

**What lied**: The tracking issue's instruction list. The orchestrator wrote
it while composing the correction after the near-miss and carried a stale
assumption about which changes had landed in master.

**Why it lied**: Branch state changes in real time. Between filing the
tracking issue and the builder picking it up, PR #4090 was closed. The
orchestrator had up-to-date information about the *intent* (revert the
phase-block model) but stale information about *where the workaround had
actually landed*.

**Who caught it**: The builder, because `git log` is a standing part of
builder preflight. The catch wasn't extra effort; it was the normal first
step.

**The teachable fact**: Orchestrator instructions can carry stale assumptions
about branch and merge state. The builder's initial `git log` / `gh pr view`
checks are the cheapest possible safety net for this category of error.

---

## Incident 4: Docs sweep catches wrong PR attribution in an orchestrator claim

**Chain**:

1. Earlier in the session, an orchestrator-authored retrospective draft
   claimed: *"#3472 `qw()` imports shipped in PR #3808."* The claim was
   repeated in dispatch prompts for multiple downstream agents that same
   session, so it was not just a one-off — it had already started to
   propagate.
2. **Docs sweep agent a9e71d80** began the 2026-04-11 docs correction wave
   ([PR #4121](https://github.com/EffortlessMetrics/perl-lsp/pull/4121)) and
   verified each cited issue/PR against GitHub state. For #3472 the agent
   ran `gh pr view 3808 --json title,closingIssues` and discovered:
   - PR #3808 is titled `fix(navigation): resolve imported function
     goto-definition (#3466)` — it fixes **#3466**, not #3472.
   - Issue #3472 (`[module-resolution] Import list symbols not extracted for
     bareword resolution`) is still **open**.

   The docs sweep corrected the attribution in the retrospective and in every
   downstream doc that had picked up the wrong claim.

**What lied**: The orchestrator's own retrospective draft. The fact was
"remembered" from the session context but misattributed to the wrong PR
number, and the wrong number was then relayed to other agents.

**Why it lied**: Session-memory claims are not verified by any upstream
layer. The orchestrator is assumed to be the authoritative source for
session state. When that source is wrong, the error compounds as it gets
relayed into multiple prompts before any downstream layer has a chance to
catch it.

**Who caught it**: The docs sweep, because verifying PR-number citations
against GitHub state was the agent's actual job. A docs-writing agent that
trusted the prompt without verification would have propagated the wrong
attribution to seven more files.

**The teachable fact**: When the orchestrator "remembers" something that
isn't true, the claim can seed multiple simultaneous agents before any of
them executes against real state. The downstream layer that first
dereferences the claim is the only catch.

---

## Generalization: Three Classes of Upstream Error

Across the four incidents, the upstream errors fall into three classes, each
caught by the same mechanism: a downstream layer that had to actually execute
against real state.

**Class 1: CLI / API claims from layers that didn't run the tool.**
Upstream layers describe what a command does based on documentation, memory,
or help-text skim. The claim sounds right. The downstream layer that has to
invoke the command discovers — on the actual help output, error message, or
exit code — that the claim was wrong. Caught by the layer that runs the
tool. (Incident 1.)

**Class 2: Project metadata errors in orchestrator-originated prompts.**
URLs, org names, PR numbers, file paths. The orchestrator treats these as
ground truth and writes them directly into prompts. There is no upstream
layer. Caught by the layer that dereferences the metadata to do its job.
(Incidents 2 and 4.)

**Class 3: Stale assumptions about branch / merge state.**
Upstream instructions were correct when composed but got stale before the
downstream layer picked them up. The downstream layer's preflight checks
(`git log`, `gh pr view`) catch it as a byproduct of normal work. (Incident
3.)

In every class, the downstream layer isn't doing extra auditing. It just
happens to need the real state to function, and the act of loading that real
state is itself the verification.

---

## Structural Implications

These are observations, not proposals. Four incidents is enough data to
recognize a pattern and suggest places to look, but not enough to prescribe
new infrastructure.

**1. Trust direction is not strict.**

The clean upstream-to-downstream trust gradient of the layered verification
protocol (scout -> accuracy -> plan-review -> build -> review -> deep-review)
describes one axis of catch. It does not describe a property of the system as
a whole. Downstream layers catch upstream errors as a side effect of
executing against real state, and this side-effect catch is apparently
load-bearing. Removing it — for example, by pre-computing state for the
downstream layer so it doesn't have to dereference anything — would probably
surface errors that are currently caught silently.

**2. Builders are a last-line-of-defense for CLI claims.**

When an upstream layer tells a builder "use flag X", the builder is often the
first layer that has to run the binary. If the upstream claim is wrong, the
builder is the only catch. This is worth being explicit about in builder
prompts: when upstream instructions cite a specific flag, command, or option,
the builder should verify the flag exists *before* editing the file that
depends on it. Treat it as a standing preflight, not a special case.

**3. Orchestrator prompts have no upstream layer.**

There is currently no "plan-reviewer for orchestrator prompts." The
orchestrator writes a prompt, the agent runs it. Factual errors in the prompt
propagate. The cheapest mitigation is a single standing instruction in
orchestrator-originated prompts for data-heavy tasks: *verify every cited file
path, URL, and PR number against real state before using it.* This is
equivalent in spirit to the research-verifier protocol tracked in #4111, but
scoped to orchestrator claims rather than external-semantics claims.

**4. The catch rate is probably higher than anyone realized.**

Four incidents in one ~6-hour session is a lot. If the pattern generalizes,
downstream-catches-upstream may be a significant fraction of all agent-level
error catches — but it is invisible unless agents explicitly report *"I
corrected the prompt/instruction before acting."* Today, most agents silently
fix upstream errors and move on. A minor prompt change — *"always report
prompt/instruction corrections explicitly in your wrapup"* — would make the
upstream error rate measurable for the first time.

---

## What This Article Is Not

This article names a single pattern and walks four evidence points. It
deliberately does not do three related things that belong in adjacent
documents.

- **Not the session retrospective.** The full 2026-04-11 wave retrospective,
  with all seven patterns, lives in the wisdom doc dispatched in
  [PR #4117](https://github.com/EffortlessMetrics/perl-lsp/pull/4117).
- **Not the layered verification protocol.** The protocol doc tracked in
  [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111)
  describes the upstream-to-downstream catch direction in detail. This
  article describes the *inversion* of that direction — a separate failure
  mode, not a revision to the protocol.
- **Not the PR #4090 case study.** The full minute-by-minute forensic of
  the false-premise cascade is in
  [PR #4127](https://github.com/EffortlessMetrics/perl-lsp/pull/4127). This
  article uses PR #4090's spinoffs (the #4100 revert, #4108 corrected fix)
  as two of its four incidents but does not retell the whole arc.

The three documents together cover what the session learned; each one stays
scoped to the angle it uniquely describes.

---

## Cross-References

- Session wisdom retrospective: [PR #4117](https://github.com/EffortlessMetrics/perl-lsp/pull/4117)
- Swarm-operations retrospective: [PR #4125](https://github.com/EffortlessMetrics/perl-lsp/pull/4125)
- PR #4090 false-premise case study: [PR #4127](https://github.com/EffortlessMetrics/perl-lsp/pull/4127)
- Layered verification protocol: [#4111](https://github.com/EffortlessMetrics/perl-lsp/issues/4111)
- Engineering-health scorecard (incident 1): [#4070](https://github.com/EffortlessMetrics/perl-lsp/issues/4070) / [PR #4124](https://github.com/EffortlessMetrics/perl-lsp/pull/4124)
- Phase-block pragma revert (incident 3): [#4100](https://github.com/EffortlessMetrics/perl-lsp/issues/4100) / [PR #4108](https://github.com/EffortlessMetrics/perl-lsp/pull/4108)
- Docs correction sweep (incident 4): [PR #4121](https://github.com/EffortlessMetrics/perl-lsp/pull/4121)
- Umbrella: [#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062)
