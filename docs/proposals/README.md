# Proposals

Proposals describe why a lane exists: the user pain, product motivation,
alternatives considered, and success criteria that make the work worth doing.
They are the PRD layer for `perl-lsp` planning.

| Layer | Owns | Must not do |
|---|---|---|
| Proposal | User problem, affected surfaces, success criteria, alternatives, non-goals, claim boundary | PR sequencing, proof command ownership, generated metric state |

## When to Add a Proposal

Add a proposal when a lane changes product direction or combines multiple
subsystems under one user-facing outcome. A proposal should make the lane useful
to maintainers, reviewers, and future agents without encoding the implementation
order.

Proposal files should use the `PLSP-PROP-####-short-name.md` pattern for
`perl-lsp` lanes. Link the proposal to specs, ADRs, implementation plans, and
status docs when those artifacts exist.

## Current Status Sources

Generated status remains the source of current facts. Proposals may link to
these files but should not copy generated tables or point-in-time counts:

- [parser accuracy next](../project/status/parser_accuracy_next.md)
- [parser status](../project/status/parser.md)
- [provider cutover](../project/status/provider_cutover.md)
- [semantic scorecard](../project/status/semantic_scorecard.md)
- [semantic shadow compare](../project/status/semantic_shadow_compare.md)
- [semantic capability dashboard](../project/status/semantic_capability_dashboard.md)
- [UX capability dashboard](../project/status/ux_capability_dashboard.md)

## Proposal Index

| Proposal | Status | Created | Title |
|----------|--------|---------|-------|
| [PLSP-PROP-0001](PLSP-PROP-0001-real-perl-editor-trust.md) | proposed | 2026-05-13 | Real Perl Editor Trust |
| [PLSP-PROP-0002](PLSP-PROP-0002-compiler-program.md) | proposed | 2026-06-21 | Repo-native compiler-program contracts |
| [PLSP-PROP-0003](PLSP-PROP-0003-spec-governance.md) | proposed | 2026-07-10 | Spec-governance via cargo-allow |

## Template

```md
# PLSP-PROP-####: Title

Status:
Owner:
Created:
Target milestone:
Linked specs:
Linked ADRs:
Linked plan:
Support/status impact:
Policy impact:

## Problem

## Users and Surfaces

## Success Criteria

## Proposed Shape

## Non-goals

## Evidence Plan
```
