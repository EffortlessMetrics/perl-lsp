# VERIFICATION_LADDER.md

Operational quick-reference for claim verification. See [layered-verification.md](../project/protocols/layered-verification.md) for theory and rationale.

## What is the ladder?

Every factual claim in a scout issue, plan-review, or PR description maps to a mandatory verification check. The ladder ensures each claim type reaches the right verifier, not just the nearest one.

## Claim-type to verifier mapping

| Claim type | Mandatory check | Fallback label if check unavailable |
|---|---|---|
| Perl language behavior (syntax, semantics, idioms) | `research-verifier` — web search against perldoc / perlmonks | `needs-research-verification` |
| LSP / DAP protocol spec (method names, capability fields) | `research-verifier` — web search against spec.lsp.dev or Microsoft DAP docs | `needs-research-verification` |
| Crate API claims (function signatures, trait impls) | `research-verifier` — docs.rs search + codebase grep | `needs-research-verification` |
| Attribution claims ("PR #N fixed this", "already shipped") | git-history check — `git log` + `gh pr view <N>` | `needs-git-history-check` |
| Logic and edge case correctness | deep-review — reviewer-deep-analyze + reviewer-deep-edges | request deep review |

## Where each check happens

- **research-verifier**: dispatched by builder (builder-self-review), plan-reviewer (plan-review-stress), or reviewer-deep (reviewer-deep-analyze) when claim-heavy criteria are met
- **git-history check**: runs inline in Attribution Check blocks in those same three command files
- **deep-review**: two-pass review gate; always required for non-docs PRs

## Related docs

- [layered-verification.md](../project/protocols/layered-verification.md) — why multiple verification lenses exist and the lens-diversity principle
- [verification.md](../project/protocols/verification.md) — CI tier definitions (Gate A/B/C)
