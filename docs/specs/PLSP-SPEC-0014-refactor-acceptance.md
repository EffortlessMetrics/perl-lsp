# PLSP-SPEC-0014: Refactor acceptance

Status: proposed
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked ADRs: none yet
Linked plan: [0.14.0 Readiness Queue](../releases/0.14.0-readiness.md)
Status impact: PR queue disposition, refactor review comments, trust-lane CI
routing, provider confidence, parser and semantic proof receipts

## Contract

A refactor PR is mergeable only when it states the behavior-preservation claim
and proves that claim at the risk level of the touched surface.

The minimal refactor review summary is:

```text
public API unchanged:
behavior unchanged:
tests proving unchanged behavior:
files moved:
semantic risk:
rollback:
```

`public API unchanged` must identify whether exported Rust APIs, LSP/DAP
protocol behavior, CLI flags, JSON/status schemas, fixtures, and policy files
are unchanged. If any public API changes, the PR is not a pure refactor and
must name the behavior or contract spec that authorizes the change.

`behavior unchanged` must name the behavior surface, not just the module that
moved. "This improves maintainability" is not a sufficient claim.

## Risk Levels

Every refactor PR must choose the highest applicable risk level.

| Risk level | Applies when | Required proof |
| --- | --- | --- |
| `leaf-crate-refactor` | Internal helper/module split in a leaf or low-fanout crate, with no provider/runtime/parser semantics changed | touched crate check, test, clippy, formatting |
| `provider-runtime-refactor` | Completion, hover, diagnostics, code actions, document links, formatting, workspace index, subprocess, or provider routing changes | touched crate check, test, clippy, relevant provider or receipt smoke |
| `dap-lsp-runtime-refactor` | LSP/DAP server lifecycle, stdio, text sync, diagnostics lifecycle, launch/config, process, or transport changes | touched crate check, test, clippy, serialized e2e or smoke where applicable |
| `parser-semantic-refactor` | Parser, lexer, AST, token, POD, module resolution, pragma, semantic analyzer, facts, or source-position internals change | parser or semantic tests, downstream consumer smoke when contracts move, status/proof receipt if behavior-adjacent |
| `control-plane-refactor` | `xtask`, scripts, workflows, gate policy, support claims, generated status, release, or proof-routing code changes | policy/gate validators, changed-script smoke, docs/status checker, storage proof when build routes change |

If a PR fits multiple rows, use the strictest row or the union of required
proof. A refactor may be classified as `risky-refactor` under
[PLSP-SPEC-0006](PLSP-SPEC-0006-pr-queue-disposition.md) until the risk level
and proof are explicit.

## Valid Refactor Claims

Valid claims are specific and falsifiable:

```text
This PR changes module boundaries only for the duplicate-hash-key lint.
The public diagnostic code, message text, ranges, and provider output are
unchanged.
```

```text
This PR moves DAP process-state helpers without changing launch, attach,
disconnect, or error-reporting behavior.
```

```text
This PR splits parser helper files without changing AST shape, token
consumption, diagnostics, or recovery behavior.
```

## Invalid Refactor Claims

Invalid claims are vague or overbroad:

```text
This improves maintainability.
```

```text
This is just SRP.
```

```text
No behavior changed.
```

Those statements may be true, but they are not enough. The PR must name the
unchanged behavior surface and the proof that would catch a hidden behavior
change.

## Acceptance

A refactor PR satisfies this spec when:

- the PR body or maintainer summary includes the minimal refactor review fields
- the highest applicable risk level is named
- public API impact is explicitly `unchanged` or tied to a separate behavior
  spec
- moved files, renamed modules, and ownership boundaries are listed
- tests or receipts prove the behavior surface named by the claim
- status or generated proof changes are regenerated rather than hand-edited
- rollback is obvious enough for a squash-merged PR
- unrelated cleanup is excluded unless needed to complete the refactor safely

Large refactors should be split when each slice can preserve behavior and prove
it independently. A broad refactor must not block a narrow correctness fix that
can merge first.

## Valid PR Shapes

Valid PRs include:

- leaf helper extraction with focused crate gates
- provider routing split with provider snapshot or receipt proof
- LSP/DAP lifecycle split with serialized smoke or e2e proof
- parser/semantic module split with behavior fixtures and downstream checks
- control-plane helper split with policy validators and storage proof
- docs-only PR that adds this acceptance contract

## Invalid PR Shapes

Invalid PRs include:

- behavior changes hidden behind refactor wording
- public API changes without an authorizing spec or migration note
- broad formatting churn mixed with logic movement
- deleting or weakening tests to make the refactor pass
- moving parser, semantic, or provider logic without a behavior smoke
- changing generated status by hand
- landing "SRP" churn without a concrete reviewability or ownership boundary
- merging a stale refactor that conflicts with already-merged correctness fixes
  when the only remaining value is churn

## Proof Commands

All refactor PRs must run:

```bash
git diff --check
./scripts/storage-doctor
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask fmt --check
```

Leaf crate refactors must also run:

```bash
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe check --all-targets -p <crate> --profile agent --locked
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe test -p <crate> --profile agent --locked
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe clippy -p <crate> --profile agent --locked -- -D warnings -A missing_docs
```

Provider and runtime refactors must add the relevant provider smoke, receipt,
or matrix check. Examples include:

```bash
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask check-provider-confidence-matrix
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask check-support-claims
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask semantic-shadow-compare --check
```

Parser or semantic refactors must run the touched parser or semantic tests and
add a parser, corpus, semantic, or status receipt when the moved code is
behavior-adjacent.

DAP/LSP runtime refactors must run the relevant serialized smoke or e2e test
binary when lifecycle, stdio, diagnostics, or transport behavior is touched.

Docs-only PRs for this spec may use:

```bash
git diff --check
MIN_FREE_GB=20 MAX_USED_PCT=95 ./scripts/cargo-safe xtask ci-hygiene check-doc-paths docs/specs
./scripts/storage-doctor
```

## Non-goals

- Do not block small correctness fixes behind broad refactors.
- Do not require every file move to become a separate PR.
- Do not forbid public API changes; require them to be explicit behavior or
  contract changes instead of hidden refactors.
- Do not replace provider, parser, edit-safety, or trust-lane specs.
- Do not promote support tiers from refactor proof alone.
- Do not define release readiness.

## Claim Boundaries

A passing refactor proof shows the named behavior surface remained stable under
the selected checks. It does not prove unrelated behavior, release readiness,
parser bucket movement, provider cutover, support-tier promotion, or global
workspace health.

If a refactor uncovers a needed behavior fix, split the behavior change into a
separate PR or clearly reclassify the PR as a behavior change with the owning
spec and regression tests.
