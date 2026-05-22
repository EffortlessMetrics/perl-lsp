# PLSP-SPEC-0023: Ambient inputs

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0009](PLSP-SPEC-0009-workspace-trust-report.md)
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0026](PLSP-SPEC-0026-determinism-receipt-v1.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: workspace trust report, module resolution status, provider
decision receipts, determinism planning, support tiers

## Current Implementation Status

Current ambient setup state is reported through the workspace trust report and
module-resolution status. Existing code already distinguishes configured roots,
`PERL5LIB`, startup `@INC`, DAP/client state, perldoc configuration, and Perl
oracle seams in several provider and runtime paths.

This spec defines the shared ambient-input contract for Real Perl Editor Trust
and future determinism receipts. It does not introduce a new runtime registry,
JSON schema, provider behavior, or subprocess behavior in the PR that adds the
spec.

## Contract

An ambient input is any fact source that can affect Perl behavior without being
an explicit source declaration in the analyzed workspace document.

Ambient inputs may be reported, labeled, redacted, and used only by surfaces
whose specs grant that authority. They must not be silently converted into
source-backed facts.

Ambient input classes include:

| Class | Examples | Default authority |
| --- | --- | --- |
| WorkspaceConfig | configured include paths, configured Perl binary, configured flags | Explicit configuration input for the configured surface. |
| SourceScopedConfig | lexical `use lib`, `no lib`, modeled FindBin paths | Source-backed resolver input with source scope. |
| ProcessEnvironment | `PERL5LIB`, `PERL5OPT`, `HOME`, local::lib activation variables | Ambient; opt-in or denied by subprocess seam. |
| InterpreterState | startup `@INC`, Perl version, core library roots | Oracle-derived ambient state. |
| ClientRuntimeState | VS Code DAP/perldoc state, launch configuration counts, path classes | Report metadata only unless promoted by behavior proof. |
| GeneratedRoots | `blib`, generated roots, build output roots | Ambient/generated; labeled and never workspace source by default. |
| ExternalOracle | `perldoc`, real Perl, CPAN-installed modules | Helper or differential oracle boundary. |
| UnknownAmbient | unclassified external influence | Fallback, block, or defer. |

## Ambient Rules

### Reporting

Ambient inputs may appear in workspace trust reports, provider decision
receipts, diagnostics explanations, module-resolution summaries, and future
determinism receipts when they are labeled with their class and authority.

Reports should prefer path classes, counts, root classes, hashes, and sanitized
labels over raw paths when raw values are not required to explain behavior.

### Provider Behavior

Provider behavior may consume ambient inputs only when a spec and support row
name that authority. If an ambient input participates in a provider decision,
the decision receipt must show the fact source, confidence or unavailability,
freshness when meaningful, fallback state, and blocker or claim boundary.

Ambient inputs must not authorize rename, safe-delete, exact generated method
body locations, or broad diagnostic suppression unless a provider-specific
promotion row and receipt explicitly allow it.

### Subprocess Seams

Subprocess seams must state which ambient environment variables are allowed,
denied, or explicitly supplied. Denied ambient inputs must not leak through by
default.

`PERL5LIB`, `PERL5OPT`, `HOME`, local::lib variables, and configured launch
environment values are distinct authorities. A surface that allows one of them
does not automatically allow the others.

### Determinism

Future determinism receipts must classify ambient inputs instead of hiding them
inside deterministic claims.

Suggested determinism projection:

```text
deterministic:
  no relevant unmodeled ambient input participates

bounded_dynamic:
  ambient inputs participate but are explicitly configured, labeled, and bounded

non_deterministic:
  ambient or dynamic behavior affects compile facts without a bounded model

unknown:
  ambient input coverage is incomplete, stale, or unclassified
```

## Valid PR Shapes

Valid PRs under this spec include:

- docs PRs that clarify ambient-input authority without behavior changes
- workspace trust report PRs that add sanitized ambient classes from already
  known state
- provider decision PRs that expose ambient blockers or fallback reasons
- subprocess seam PRs that deny, allow, or explicitly supply one ambient input
  class with tests
- determinism PRs that classify ambient inputs in receipts

## Invalid PR Shapes

Invalid PRs include:

- treating ambient inputs as workspace source
- hiding `PERL5LIB`, startup `@INC`, DAP metadata, generated roots, perldoc, or
  real Perl oracle output inside exact provider claims
- probing Perl, running `perldoc`, starting DAP, inspecting debug sessions, or
  scanning the workspace from report-only surfaces
- exposing raw secrets, tokens, private environment values, or unnecessary raw
  launch paths
- promoting support tiers from ambient reporting alone
- using unclassified ambient evidence to authorize edits

## Acceptance

A PR satisfies this spec when:

- every touched ambient input has a class and authority
- report-only surfaces stay read-only and sanitized
- subprocess seams preserve explicit allow/deny behavior
- provider receipts expose ambient fallback or blocker state when relevant
- edit-producing providers treat ambient-only evidence as fallback, preview, or
  blocked unless a promotion row proves otherwise
- determinism-facing work records ambient inputs instead of erasing them
- support claims do not outgrow receipts

## Proof Commands

Docs-only PRs for this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask check-support-claims
cargo xtask check-provider-confidence-matrix
git diff --check
```

Runtime or provider PRs must also run the focused tests for the changed
subprocess seam, provider, trust-report, diagnostic, module-resolution, or
determinism surface.

## Non-goals

- No new ambient-input registry from this spec alone.
- No provider behavior change.
- No workspace trust report probing.
- No DAP launch behavior change.
- No determinism receipt implementation.
- No real Perl replacement claim.

## Claim Boundaries

This spec may claim that ambient inputs must be labeled, bounded, and kept out
of exact source-backed claims. It may not claim that every ambient input is
fully modeled, deterministic, safe for edits, or available to every provider.
