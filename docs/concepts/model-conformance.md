# Model Conformance

## The isomorphism

A system's code failure mode (duplication → divergence → one copy silently wrong) and
its agent-fleet failure mode (multiple agents operating on divergent substrate models,
one silently wrong) are **isomorphic**. The same fix applies to both.

**In code**: two copies of the same logic diverge over time. Both look plausible. One
encodes the bug. The fix is to centralize the source of truth — but *prove agreement
first* before centralizing, or the refactor silently promotes the broken copy.

**In a fleet**: two agents (or two orchestrator sessions) operate on different models
of the substrate. Both look plausible. One has the wrong base ref, wrong CI timing
assumption, or stale merge-queue config. The fix is to centralize the model — but
*verify conformance first*, or the encoding silently promotes the stale model.

## The conformance-before-centralize discipline

In both cases, the correct order is:

1. **Build the conformance matrix** — enumerate all copies and what they claim
2. **Run the matrix** — verify that all copies agree on the behavior that matters
3. **Find the outlier** — the copy that differs is the silently broken one
4. **Centralize** — now that the broken copy is identified, merge into the correct one

Skipping step 2 makes centralization a gamble: if the consolidation target is the wrong
copy, every consumer silently adopts the defect.

## Applied to code

When multiple modules independently implement the same logic:

- Map every consumer to its implementation
- Verify each implementation produces the same output on shared test inputs
- Identify any that diverge — those are the bugs, independent of which "looks right"
- Centralize around the correct implementation with the others' consumers migrated

The matrix step is cheap. The divergence it finds is valuable precisely because the
diverging copy *looked* correct in isolation — static inspection alone would not have
found it.

## Applied to agent models

When multiple agents (or orchestrator sessions) must model the same substrate:

- Identify what each agent currently believes about the substrate (CI timing, branch
  protection rules, required checks, concurrency model, cache TTL)
- Verify agreement — where do they differ?
- Find the session whose model is inconsistent with current observed behavior — that is
  the stale/wrong model
- Encode the correct model centrally so every subsequent session inherits it

A fleet where half the agents believe main is stable (single-thread assumption) and
half know it moves will produce incoherent serialization decisions — some agents will
hold work futilely; others will race correctly. The inconsistency is invisible until
you map the models and compare them.

## When one copy is "more right"

The conformance matrix does not always reveal a clear winner — sometimes all copies are
partially wrong in different ways. In that case:

- The canonical source of truth is **observed behavior**, not any copy's claim
- Run the behavior (the actual CI check, the actual merge, the actual cache hit rate)
  and reconcile each copy's model against the observation
- The copy closest to observed behavior is the least-wrong starting point for
  centralization

## Caution: centralize to the right copy

The most dangerous mistake is centralizing around the wrong copy when both look
plausible. Always prefer the copy that can be verified against an external oracle
(the actual CI log, the actual merge-commit timing, the actual token count). If no
external oracle is available, add one before centralizing.

## Relation to other patterns

- **Orchestrator substrate model** (`orchestrator-substrate-model.md`) — the fleet
  analogue: when agents diverge on the substrate model, this conformance discipline
  finds the inconsistency before it is silently promoted
- **Shift-left ladder** (`shift-left-ladder.md`) — conformance checks belong on the
  "spec acceptance criteria" rung; running them before the refactor prevents
  post-refactor debugging
- **Re-create over untangle** (`re-create-over-untangle.md`) — if conformance reveals
  that a branch's model is irrecoverably wrong, re-creation from a verified clean model
  is cheaper than untangling
