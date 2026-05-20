# perl-lsp-swarm Operating Model

`perl-lsp-swarm` is the active execution repo for Real Perl Editor Trust work.
`perl-lsp` remains the canonical release, history, and package-lineage repo until
curated sync and release PRs promote swarm work back upstream.

This document is a control-plane contract. It does not change provider behavior,
CI workflows, release automation, labels, or package publication.

## Repo Roles

| Repo | Role | Allowed work |
|---|---|---|
| `perl-lsp-swarm` | Active development execution repo | Agent lanes, promotion-ledger work, proof receipts, cleanup trains, spec hardening, compiler substrate work |
| `perl-lsp` | Release lineage and curated upstream repo | Release sync PRs, history preservation, user-facing package lineage, emergency release fixes |

Default routing rule:

```text
All new development targets perl-lsp-swarm.
perl-lsp receives curated sync and release-lineage PRs only.
```

## Active Manifest

The machine-readable current lane lives in
[`.perl-lsp/goals/active.toml`](../../.perl-lsp/goals/active.toml).

The manifest records:

- current repo role
- lane WIP caps
- trust, substrate, and reliability lane ownership
- next work queues
- proof commands for this control-plane boundary

It points at status and spec documents instead of copying generated status
tables.

## Lanes

| Lane | WIP cap | Owns | Rule |
|---|---:|---|---|
| Trust | 2 PRs | Provider promotion ledger, Real Perl Editor Trust v1 boundary, workspace symbols, semantic tokens, rename, safe-delete, diagnostic explanations, workspace trust report | No broadening. Every provider-facing PR names promotion, fallback, blocker, and receipt boundaries. |
| Compiler substrate | 2 PRs | Lexer/parser/proptest, constants, prototypes, barewords, PIR, determinism prep, oracle prep | No provider cutover unless the trust lane explicitly promotes a fact class. |
| Reliability | 4 PRs | Fuzzing, E2E diagnostics, DevEx, docs, SRP refactors, coverage, policy cleanup, published API hygiene | Merge clean leaf work. Escalate trust-adjacent surfaces. |

Work outside those caps parks unless it fixes a red gate.

## Trust-Lane PR Discipline

Every main trust-lane PR must name:

```text
one fact class
one provider surface
one promotion rule
one fallback rule
one blocker rule
one receipt
```

If a PR cannot name those boundaries, it is reliability or substrate work, not
main-lane promotion work.

## High-Scrutiny Surfaces

These surfaces cannot be treated as routine cleanup:

- rename
- safe-delete
- code actions
- subprocess runtime
- URI and path normalization
- module path resolution
- LSP runtime state
- DAP launch or DAP process state
- published public APIs
- provider promotion rows

Refactors touching those surfaces need explicit behavior boundaries and proof.

## Current Control-Plane Boundary

Real Perl Editor Trust stays bounded by the existing specs and status docs:

- [Real Perl Editor Trust v1 boundary](../specs/PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [Provider decision receipt v1](../specs/PLSP-SPEC-0016-provider-decision-receipt-v1.md)
- [Fact provenance and source backing](../specs/PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [Edit authorization contract](../specs/PLSP-SPEC-0018-edit-authorization-contract.md)
- [Semantic token class promotion contract](../specs/PLSP-SPEC-0019-semantic-token-class-promotion-contract.md)
- [Workspace symbol generated-label contract](../specs/PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md)
- [Diagnostic explanation v1](../specs/PLSP-SPEC-0021-diagnostic-explanation-v1.md)
- [Module path authority](../specs/PLSP-SPEC-0022-module-path-authority.md)
- [Ambient inputs](../specs/PLSP-SPEC-0023-ambient-inputs.md)
- [Framework fact adapters](../specs/PLSP-SPEC-0024-framework-fact-adapters.md)
- [PIR v0](../specs/PLSP-SPEC-0025-pir-v0.md)
- [Real Perl Editor Trust dashboard](../project/status/real_perl_editor_trust_v1.md)
- [Provider promotion ledger](../project/status/provider_promotion_ledger.md)

## Next Control-Plane PRs

After this manifest PR, keep the next control-plane slices separate:

1. Add the swarm PR template and review rules.
2. Mark `perl-lsp` as release-lineage only.
3. Define the curated sync protocol from `perl-lsp-swarm` to `perl-lsp`.

Those follow-ups should not be bundled into this PR.
