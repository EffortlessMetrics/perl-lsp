# Swarm Review Rules

`perl-lsp-swarm` is the active execution repo. Reviews route work by lane and
risk before judging implementation details.

These rules are control-plane guidance only. They do not change provider
behavior, CI workflows, branch protection, labels, or release automation.

## Lanes

Every PR declares one lane.

| Lane | Owns | Review rule |
|---|---|---|
| Trust | Provider promotion ledger, Real Perl Editor Trust boundary, workspace symbols, semantic tokens, rename, safe-delete, diagnostic explanations, workspace trust report | No broadening. Name promotion, fallback, blocker, and receipt boundaries for any provider-facing change. |
| Substrate | Lexer/parser/proptest, constants, prototypes, barewords, PIR, determinism prep, oracle prep | No provider cutover unless the trust lane explicitly promotes a fact class. |
| Reliability | Fuzzing, E2E diagnostics, DevEx, docs, SRP refactors, coverage, policy cleanup, published API hygiene | Merge clean leaf work. Escalate trust-adjacent surfaces instead of treating them as cleanup. |

## WIP Caps

Use the active manifest caps:

```text
trust: 2 PRs
substrate: 2 PRs
reliability: 4 PRs
```

Work outside those caps parks unless it fixes a red gate.

## Trust-Lane Requirements

Every trust-lane PR must name:

```text
one fact class
one provider surface
one promotion rule
one fallback rule
one blocker rule
one receipt
```

If a PR cannot name those boundaries, route it as substrate or reliability work.

## Behavior States

Every PR declares one behavior state:

- `no behavior change`: docs, policy, tests, or refactor-only proof.
- `preview only`: visible output or receipts without edit-producing behavior.
- `scoped pilot`: bounded live behavior behind explicit promotion conditions.
- `live behavior change`: user-facing behavior changes outside preview-only paths.

Live behavior changes need the narrowest useful proof first, followed by the
repo gate required for the risk surface.

## High-Scrutiny Surfaces

These surfaces are never routine cleanup:

- rename
- safe-delete
- code actions
- subprocess runtime
- URI and path normalization
- module path resolution
- LSP runtime state
- DAP launch or DAP process state
- workspace configuration
- published public APIs
- parser or lexer core
- provider promotion rows

A PR touching any of these surfaces must state the behavior boundary and proof.

## Merge Quickly If Green

Small leaf work can move quickly when CI is green and the PR is scoped:

- narrow parser or lexer tests
- URI boundary tests
- TDD support fixes
- xtask helper cleanup
- docs-only cleanup outside trust claims

## Review Carefully

Review carefully before merging:

- code actions
- subprocess runtime
- DAP process, launch, or state handling
- workspace configuration
- command timeout behavior
- public token or parser APIs
- path or module resolution

If proof is thin, park the PR or require a narrower receipt.

## Verification

Prefer the cheapest proof that exercises the change first. For trust-adjacent
changes, require CI green or an explicit risk acceptance before merge.

If `cargo-safe` is blocked by local storage guard state, acceptable alternatives
before merge are:

- hosted CI green
- rerun with corrected local storage settings
- explicit cleanup-only risk acceptance

Do not use cleanup-only acceptance for trust-lane promotion, edit-producing
providers, subprocess boundaries, path/module resolution, DAP launch behavior,
or public APIs.
