# Specs

Specs define what must be true for a behavior, status surface, or proof lane.
They are contracts for acceptance, proof requirements, and claim boundaries.

See [SPEC_SYSTEM.md](../reference/SPEC_SYSTEM.md) for the full
source-of-truth stack and agent workflow.

| Layer | Owns | Must not do |
|---|---|---|
| Spec | Behavior contract, acceptance criteria, proof requirements, status interpretation, claim limits | Product motivation, broad roadmap, PR sequence, active queue ownership |

## When to Add a Spec

Add a spec when future work needs a durable contract that reviewers and agents
can apply across more than one PR. A spec should make it clear how to decide
whether a change satisfies the lane without requiring chat history.

Spec files for `perl-lsp` lane work should use the
`PLSP-SPEC-####-short-name.md` pattern. Specs should link to generated status
docs and human-owned dashboards, but they should not hand-edit or duplicate
generated sections.

## Acceptance and Proof

Each spec should include:

- the contract being enforced
- valid and invalid PR shapes when useful
- proof commands or status checks
- explicit non-goals
- claim boundaries for docs, releases, and user-facing behavior

## Current Status Sources

Generated status is current state, not spec text. Link to these files instead
of copying generated values:

- [parser accuracy next](../project/status/parser_accuracy_next.md)
- [parser status](../project/status/parser.md)
- [provider cutover](../project/status/provider_cutover.md)
- [semantic scorecard](../project/status/semantic_scorecard.md)
- [semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [semantic capability dashboard](../project/status/semantic_capability_dashboard.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)

## Template

```md
# PLSP-SPEC-####: Title

Status:
Owner:
Linked proposal:
Linked ADRs:
Linked plan:
Status impact:

## Contract

## Acceptance

## Proof Commands

## Non-goals

## Claim Boundaries
```
