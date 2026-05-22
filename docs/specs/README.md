# Specs

Specs define what must be true for a behavior, status surface, or proof lane.
They are contracts for acceptance, proof requirements, and claim boundaries.

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

## PLSP Spec Catalog

- [PLSP-SPEC-0001: Parser compatibility bucket closeout](PLSP-SPEC-0001-parser-compatibility-bucket-closeout.md)
- [PLSP-SPEC-0002: Provider confidence receipts](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0003: Real workspace editor baseline](PLSP-SPEC-0003-real-workspace-editor-baseline.md)
- [PLSP-SPEC-0004: Corpus receipt freshness](PLSP-SPEC-0004-corpus-receipt-freshness.md)
- [PLSP-SPEC-0005: Receiver expression facts](PLSP-SPEC-0005-receiver-expression-facts.md)
- [PLSP-SPEC-0006: PR queue disposition](PLSP-SPEC-0006-pr-queue-disposition.md)
- [PLSP-SPEC-0007: Receiver-fact completion](PLSP-SPEC-0007-receiver-fact-completion.md)
- [PLSP-SPEC-0008: Edit-producing provider safety](PLSP-SPEC-0008-edit-producing-provider-safety.md)
- [PLSP-SPEC-0009: Workspace trust report](PLSP-SPEC-0009-workspace-trust-report.md)
- [PLSP-SPEC-0010: Support claim map](PLSP-SPEC-0010-support-claim-map.md)
- [PLSP-SPEC-0011: Trust-lane CI routing](PLSP-SPEC-0011-trust-lane-ci-routing.md)
- [PLSP-SPEC-0012: User-facing trust surfaces](PLSP-SPEC-0012-user-facing-trust-surfaces.md)
- [PLSP-SPEC-0013: Agent build storage and gates](PLSP-SPEC-0013-agent-build-storage-and-gates.md)
- [PLSP-SPEC-0014: Refactor acceptance](PLSP-SPEC-0014-refactor-acceptance.md)
- [PLSP-SPEC-0015: Real Perl Editor Trust v1 boundary](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0016: Provider decision receipt v1](PLSP-SPEC-0016-provider-decision-receipt-v1.md)
- [PLSP-SPEC-0017: Fact provenance and source backing](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0018: Edit authorization contract](PLSP-SPEC-0018-edit-authorization-contract.md)
- [PLSP-SPEC-0019: Semantic token class promotion contract](PLSP-SPEC-0019-semantic-token-class-promotion-contract.md)
- [PLSP-SPEC-0020: Workspace symbol generated-label contract](PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md)
- [PLSP-SPEC-0021: Diagnostic explanation v1](PLSP-SPEC-0021-diagnostic-explanation-v1.md)
- [PLSP-SPEC-0022: Module path authority](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023: Ambient inputs](PLSP-SPEC-0023-ambient-inputs.md)
- [PLSP-SPEC-0024: Framework fact adapter contract](PLSP-SPEC-0024-framework-fact-adapters.md)
- [PLSP-SPEC-0025: PIR v0 contract](PLSP-SPEC-0025-pir-v0.md)
- [PLSP-SPEC-0026: Determinism receipt v1](PLSP-SPEC-0026-determinism-receipt-v1.md)
- [PLSP-SPEC-0027: Differential real-Perl oracle contract](PLSP-SPEC-0027-differential-real-perl-oracle.md)

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
- [receiver facts](../project/status/receiver_facts.md)
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
