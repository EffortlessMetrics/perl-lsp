# PLSP-SPEC-0027: Differential real-Perl oracle contract

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
- [PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)
- [PLSP-SPEC-0026](PLSP-SPEC-0026-determinism-receipt-v1.md)
Linked ADRs:
- [PLSP-ADR-0001](../adr/PLSP-ADR-0001-generated-status-is-control-plane.md)
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: compiler facts, parser status, determinism receipts, future
oracle receipts, support tiers
Owner issue: [#8199](https://github.com/EffortlessMetrics/perl-lsp/issues/8199)

## Current Implementation Status

The differential real-Perl oracle is a planning contract. The repo already has
source-backing, module-path authority, ambient-input, provider-decision, PIR,
and determinism receipt contracts that define the fact classes an oracle may
compare.

This spec defines the oracle contract. The repo now has a declaration-only
fixture manifest and schema plus a receipt schema and schema-shape validator,
but no oracle runner, subprocess seam, real-Perl execution path, workspace
probe, provider behavior, release-lineage sync, support-tier promotion,
corpus/parser bucket movement, or conformance claim.

Current evidence remains in:

- [Real Perl Editor Trust dashboard](../project/status/real_perl_editor_trust_v1.md)
- [compiler fact substrate](../project/status/compiler_facts.md)
- [module resolution status](../project/status/module_resolution.md)
- [provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [support tiers](../project/status/SUPPORT_TIERS.md)
- [determinism receipt v1](PLSP-SPEC-0026-determinism-receipt-v1.md)
- [oracle fixture manifest schema](../../schemas/oracle_fixture_manifest.v1.schema.json)
- [oracle fixture manifest](../../crates/perl-corpus/fixtures/differential_oracle/manifest.json)
- [oracle receipt schema](../../schemas/oracle_receipt.v1.schema.json)
- [active goal manifest](../../.perl-lsp/goals/active.toml)

Current next work is not stored here; see the routing dashboard and active goal.

## Contract

A differential oracle compares Rust-native compiler facts against observations
from real Perl for declared fixtures. It is a conformance and promotion-proof
surface, not an editor-runtime dependency and not a source of truth for live
workspace requests.

Differential oracle receipts must:

- identify the receipt schema or manifest version
- identify the comparison class
- identify the fixture or source snapshot under test
- identify the Rust compiler fact extractor and version when available
- identify the Perl interpreter, invocation mode, and declared environment
- classify module-path authority, ambient inputs, generated inputs, dynamic
  boundaries, stale state, and unsupported effects
- compare normalized facts rather than raw process output when possible
- preserve source ranges, fact provenance, confidence, freshness, and fallback
  state from the Rust fact model
- classify disagreements with a known result kind
- say whether a disagreement blocks promotion, records a known limitation, or
  remains unknown
- redact paths, environment values, launch payloads, and private fixture data
  unless a fixture explicitly declares them as public
- state whether provider behavior changed

Differential oracle receipts must not:

- run arbitrary user workspaces from editor request paths
- make Perl execution a dependency of normal completion, hover, navigation,
  diagnostics, rename, safe-delete, symbols, tokens, or trust-report requests
- treat `PERL5LIB`, startup `@INC`, DAP metadata, perldoc output, or real Perl
  observations as workspace source
- hide ambient or unbounded inputs inside source-backed claims
- promote provider behavior, support tiers, PIR, determinism, or runtime support
  from oracle agreement alone
- imply full CPAN conformance, full Perl compatibility, or a Rust Perl runtime
  replacement claim

## Required Comparison Classes

The first oracle classes are:

| Class | Compares | Promotion handling |
| --- | --- | --- |
| PackageSubTable | packages, named subs, source ranges, stash entries | Blocks exact source-backed promotion when Rust facts disagree with bounded real-Perl observations. |
| ImportExport | import specs, export sets, visible symbols | Blocks import/export provider promotion when disagreement affects a promoted source-backed claim. |
| IsaComposition | `@ISA`, inheritance facts, role/composition facts when modeled | Blocks inheritance-backed claims unless dynamic or unsupported boundaries are labeled. |
| ConstantPrototype | constants, prototype table entries, compile effects | Blocks constant/prototype provider or diagnostic promotion when source-backed facts disagree. |
| FrameworkGeneratedMember | generated members from declared framework adapters | May support labeled generated facts only when source-backed declaration anchors and framework policies agree. |
| CompileEffect | modeled compile-time effects and dynamic boundaries | Feeds determinism and PIR planning; does not authorize provider behavior by itself. |

Additional classes may be added only when the PR names the fact class,
comparison rule, fallback rule, blocker rule, receipt, and support claim
boundary.

## Fixture Selection

Oracle fixtures must be declared before they are used as proof. A fixture
declaration must name:

- fixture identity and source snapshot
- public or redacted path class
- expected Perl version constraints when relevant
- declared module roots and include-path authority
- declared environment variables or explicit denials
- generated roots and framework adapters, if any
- dynamic boundaries that are expected rather than failures
- unsupported syntax, effects, or runtime dependencies
- comparison classes covered by the fixture

Pull-request gates must use checked-in fixtures or explicitly assigned oracle
fixtures. They must not execute arbitrary user workspaces, scan undeclared
workspace roots, or depend on undeclared local CPAN state.

Dynamic or unsupported fixtures are valid when their purpose is to prove that
the Rust compiler reports a boundary. They are not valid proof for exact
source-backed promotion.

## Environment Authority

Oracle runs are hermetic only when the receipt can name every relevant input.

Required environment handling:

- `PERL5LIB`, `PERL5OPT`, local::lib variables, and other Perl startup inputs
  are denied by default unless declared by the fixture.
- System `@INC` and core library roots are ambient inputs unless the fixture
  pins and reports them.
- Configured include paths use the module-path authority contract.
- DAP `includePaths` are report/config metadata only unless a separate cutover
  proof grants authority.
- `perldoc` remains a helper/oracle boundary, not compiler truth.
- Real Perl remains a differential oracle, not an editor-runtime dependency.

Receipts must redact raw private paths and environment values unless the
fixture explicitly marks them public test data.

## Disagreement Classification

Every comparison result must use one of these result classes:

```text
oracle_agrees
compiler_missing
compiler_extra
range_mismatch
provenance_mismatch
confidence_or_freshness_mismatch
dynamic_or_unsupported
oracle_ambient_unbounded
stale_or_partial
unknown
```

Promotion blockers:

- `compiler_missing`, `compiler_extra`, `range_mismatch`, and
  `provenance_mismatch` block promotion for the affected exact source-backed
  fact class.
- `confidence_or_freshness_mismatch` blocks edit-producing and exact provider
  behavior until the confidence/freshness contract is repaired.
- `oracle_ambient_unbounded` blocks deterministic, repeatable, or conformance
  claims for that comparison.
- `stale_or_partial` blocks promotion unless the stale or partial state is the
  explicit subject of a fallback receipt.
- `unknown` blocks promotion.

Known limitation handling:

- `dynamic_or_unsupported` may become a known limitation when the dynamic or
  unsupported boundary is labeled, source-backed where possible, and outside the
  fact class being promoted.
- `oracle_agrees` supports a promotion only when the provider promotion ledger,
  support claim map, and provider-specific receipt also allow that fact class.

## Valid PR Shapes

Valid PRs under this spec include:

- docs PRs that define oracle comparison rules and claim boundaries
- fixture-manifest PRs that declare oracle fixtures without running Perl from
  editor request paths
- schema PRs that add oracle receipt shape validation
- runner PRs that execute only declared fixtures behind explicit commands
- shadow receipt PRs that compare Rust facts and real-Perl observations without
  changing provider behavior
- disagreement-report PRs that classify oracle mismatches as blockers, known
  limitations, or unknowns

Every valid PR must say whether it changes docs, schemas, fixture manifests,
runner behavior, subprocess seams, provider behavior, parser/corpus status,
determinism receipts, support claims, or release-lineage sync.

## Invalid PR Shapes

Invalid PRs include:

- making real Perl execution a dependency of normal editor requests
- running undeclared workspaces or arbitrary user code as an oracle
- promoting diagnostics, completion, navigation, semantic tokens, rename,
  safe-delete, workspace symbols, PIR, determinism, or support tiers from
  oracle agreement alone
- treating ambient `PERL5LIB`, startup `@INC`, perldoc, DAP metadata, or CPAN
  installation state as workspace source
- hiding unsupported effects, generated/no-source facts, dynamic requires,
  symbolic refs, typeglobs, `AUTOLOAD`, or stale facts
- exposing raw secrets, private environment values, raw launch payloads, or
  unnecessary raw paths
- bundling oracle work with unrelated parser, provider, refactor, release, or
  source-sync changes

## Acceptance

A PR satisfies this spec when:

- it names the oracle comparison class
- it names the source snapshot or fixture identity
- it names module-path and environment authority
- it preserves Rust fact provenance, confidence, freshness, fallback, and
  source-backing state
- it classifies every disagreement with a known result class
- it states which disagreements block promotion and which are known limitations
- it keeps real Perl out of live editor request paths
- it keeps provider behavior unchanged unless a separate promotion row and
  provider receipt explicitly allow the fact class
- it keeps support-tier wording within current receipts

## Proof Commands

Docs-only PRs for this spec may use:

```bash
rtk cargo xtask check-active-goal-manifest
rtk cargo xtask ci-hygiene check-doc-paths docs/specs
rtk cargo xtask ci-hygiene check-doc-paths docs/project/status
rtk cargo xtask check-support-claims
rtk cargo xtask check-provider-confidence-matrix
rtk git diff --check
```

Schema, fixture-manifest, runner, subprocess, provider, parser/corpus,
determinism, or support-claim PRs must also run the focused checks for the
changed surface.

Receipt-schema PRs should also use:

```bash
rtk cargo test -p xtask --profile agent --locked oracle_receipt_schema -- --nocapture
rtk cargo xtask check-oracle-receipt-schema
rtk powershell -NoProfile -Command 'Get-Content schemas/oracle_receipt.v1.schema.json -Raw | ConvertFrom-Json | Out-Null'
```

Declaration-only fixture-manifest PRs should also use:

```bash
rtk cargo test -p xtask --profile agent --locked oracle_fixture_manifest -- --nocapture
rtk cargo xtask check-oracle-fixture-manifest
rtk powershell -NoProfile -Command 'Get-Content schemas/oracle_fixture_manifest.v1.schema.json -Raw | ConvertFrom-Json | Out-Null; Get-Content crates/perl-corpus/fixtures/differential_oracle/manifest.json -Raw | ConvertFrom-Json | Out-Null'
```

## Non-goals

- No oracle runner from this spec alone.
- No oracle runner from the fixture manifest.
- No Perl execution.
- No workspace probing.
- No editor-runtime dependency on Perl.
- No provider behavior change.
- No parser/corpus bucket movement.
- No PIR implementation.
- No determinism claim.
- No support-tier promotion.
- No release-lineage sync.
- No full CPAN, full Perl compatibility, or Rust Perl runtime replacement claim.

## Claim Boundaries

This spec may claim that future differential oracle work must compare declared
Rust facts and real-Perl observations through bounded fixture, environment,
provenance, disagreement, and promotion rules. It may not claim that an oracle
runner exists, that real Perl is used by live editor requests, that any provider
behavior has changed, that determinism is proven, that support tiers have moved,
or that Rust-native Perl replacement behavior is implemented.
