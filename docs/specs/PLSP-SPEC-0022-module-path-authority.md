# PLSP-SPEC-0022: Module path authority

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0009](PLSP-SPEC-0009-workspace-trust-report.md)
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: module resolution status, workspace trust report, provider
confidence matrix, support tiers, determinism planning

## Current Implementation Status

The current module-resolution rail is tracked in
[module_resolution.md](../project/status/module_resolution.md). That status
page owns current consumer conformance, include-root classification, and
receipt history.

This spec defines authority rules for module paths and `@INC` inputs. It does
not change module resolution, DAP launch behavior, subprocess seams, or support
tiers in the PR that adds the spec.

## Contract

Module-path authority is the right of an input to affect a resolver,
subprocess, provider answer, trust report, or future determinism receipt.
Inputs with similar path shapes do not have the same authority.

Every module-path input must be classified before it can influence behavior:

| Input | Authority |
| --- | --- |
| Workspace `perl.workspace.includePaths` | Compiler and LSP resolver input inside the workspace configuration boundary. |
| Lexical `use lib` / `no lib` | Source-backed, position-scoped resolver input. |
| FindBin-derived lexical paths | Source-backed, file-scoped resolver input when the pattern is modeled. |
| `PERL5LIB` | Ambient input, visible only when the configured surface opts into it. |
| Interpreter startup `@INC` | Ambient oracle input, visible only when the configured surface opts into it. |
| DAP `includePaths` | DAP report/config metadata only unless a separate behavior receipt promotes native DAP module-path authority. |
| Launch `env.PERL5LIB` | Current DAP launch/syntax-check module-path authority for launch subprocess behavior. |
| `perldoc` | Helper/oracle boundary, not compiler truth. |
| Real Perl subprocesses | Differential oracle boundary, not editor-runtime dependency. |

No provider, report, or future compiler surface may silently treat an ambient,
report-only, or oracle input as workspace source.

## Authority Rules

### Workspace Include Paths

Configured include paths are explicit workspace configuration. They may
participate in LSP resolver behavior, provider receipts, and compiler
environment facts when the receiving surface is already allowed to consume
workspace configuration.

They must remain labeled as configured roots in receipts and reports. They are
not source-backed declarations, and they do not prove that a symbol is defined
in workspace source.

### Lexical Module Paths

Lexical `use lib`, `no lib`, and modeled FindBin-derived paths are source-backed
resolver inputs. Their authority is position-scoped and must preserve
`no lib` cancellation semantics.

These inputs may support module-resolution providers when the resolver uses the
current document position. They must not be cached or lifted into a
workspace-wide truth without preserving the source anchor and scope.

### PERL5LIB

`PERL5LIB` is an ambient input. It may affect module resolution only for
surfaces whose configuration explicitly opts into that authority.

Receipts and reports must label `PERL5LIB` as ambient. It must not be merged
into workspace source facts, silently used by subprocess oracles that disabled
it, or used to promote provider support without a receipt that names the
authority.

### Interpreter Startup `@INC`

Interpreter startup `@INC` is an oracle-derived ambient input. It may affect
module resolution only when the configured surface opts into system include
roots.

Startup `@INC` probes are subprocess seams. Their failures must be explicit and
fail closed. Startup `@INC` does not prove source-backed symbols or workspace
ownership.

### DAP Include Paths

DAP `includePaths` are report/config metadata by default. They may be counted,
classified, and displayed by the workspace trust report, but they are not
native `@INC` authority for syntax-check or debug-launch subprocess behavior
unless a separate DAP behavior receipt promotes that fact class.

Until such a receipt exists, launch `env.PERL5LIB` remains the current
module-path authority for DAP launch and syntax-check subprocess behavior.

### Perldoc And Real Perl

`perldoc` and real Perl subprocesses are helper/oracle seams. They may support
documentation lookup, setup reporting, or differential conformance receipts
when their specs allow it.

They must not become implicit editor-runtime dependencies, source-backed facts,
or provider cutover evidence without a separate receipt.

## Valid PR Shapes

Valid PRs under this spec include:

- docs PRs that clarify module-path authority without behavior changes
- receipt PRs that prove one resolver consumer respects an existing authority
- DAP behavior PRs that explicitly promote or defer one DAP module-path fact
  class
- workspace trust report PRs that report sanitized path classes without
  changing resolver behavior
- determinism PRs that classify module roots and ambient inputs

Every valid PR must say whether it changes docs, receipts, resolver behavior,
DAP launch behavior, subprocess seams, trust-report rendering, or support
claims.

## Invalid PR Shapes

Invalid PRs include:

- treating DAP `includePaths` as native `@INC` authority from report metadata
  alone
- treating `PERL5LIB` or system `@INC` as workspace source
- using ambient paths to authorize rename, safe-delete, exact generated
  locations, or diagnostic suppression without provider-specific proof
- running Perl, `perldoc`, DAP, or workspace scans from explanation-only or
  report-only surfaces
- promoting module-resolution support tiers from documentation alone
- broadening provider behavior from include-root plumbing without a promotion
  ledger row and receipt

## Acceptance

A PR satisfies this spec when:

- every touched module-path input is classified by authority
- ambient inputs remain labeled as ambient
- report-only metadata does not change resolver or subprocess behavior
- DAP launch and syntax-check authority remains explicit
- subprocess oracle seams preserve their opt-in environment rules
- provider decisions and trust reports expose the authority boundary when
  module paths participate
- support-tier wording follows receipts rather than inferred setup state

## Proof Commands

Docs-only PRs for this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Resolver, DAP, trust-report, or provider PRs must also run the focused tests
for the touched surface and any provider-ledger checks required by the
promotion row.

## Non-goals

- No module-resolution behavior change from this spec alone.
- No DAP launch or syntax-check behavior change.
- No workspace trust report probing.
- No `perldoc` or real-Perl subprocess promotion.
- No provider cutover or support-tier promotion.
- No determinism receipt implementation.

## Claim Boundaries

This spec may claim that module-path inputs have explicit authority classes. It
may not claim that all module resolution is complete, DAP `includePaths` are
native `@INC` authority, ambient roots are workspace source, or real Perl is an
editor-runtime dependency.
