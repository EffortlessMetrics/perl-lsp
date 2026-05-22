# PLSP-SPEC-0026: Determinism receipt v1

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
- [PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: Real Perl Editor Trust dashboard, compiler facts, module
resolution status, future PIR receipts, future differential oracle receipts

## Current Implementation Status

Determinism receipt v1 is a planning contract. The repo already has source
backing, module-path authority, ambient-input, PIR, provider-decision, and
workspace-trust contracts that define the input classes determinism receipts
must report.

This spec defines the receipt contract. It does not add a receipt generator,
schema file, cache, PIR implementation, runtime probe, provider behavior,
release-lineage sync, support-tier promotion, or determinism claim.

Current evidence remains in:

- [Real Perl Editor Trust dashboard](../project/status/real_perl_editor_trust_v1.md)
- [compiler fact substrate](../project/status/compiler_facts.md)
- [module resolution status](../project/status/module_resolution.md)
- [provider confidence matrix](../project/status/provider_confidence_matrix.md)
- [support tiers](../project/status/SUPPORT_TIERS.md)
- [active goal manifest](../../.perl-lsp/goals/active.toml)

Current next work is not stored here; see the routing dashboard and active goal.

## Contract

A determinism receipt explains whether one compiler/editor proof run was bounded
by declared inputs. It is a support and proof artifact, not an assertion that
Perl behavior is fully deterministic or that the editor understands all runtime
behavior.

Determinism receipt v1 must:

- identify the receipt schema version
- identify the surface being measured
- identify the source snapshot or fixture identity
- classify every input that affected the result
- preserve fact provenance, confidence, freshness, and source-backing state
- label ambient, generated, dynamic, stale, partial, and unknown boundaries
- report cache, index, and open-document state when they affect the result
- report whether the proof is repeatable under the declared input set
- explain fallback, blocker, and unknown states in plain language
- state whether provider behavior changed

Determinism receipt v1 must not:

- run Perl, `perldoc`, DAP, or application code from receipt-only surfaces
- scan the workspace from explanation-only or report-only commands
- hide `PERL5LIB`, startup `@INC`, generated roots, caches, or open documents
  inside source-backed claims
- treat PIR coverage as proof of deterministic Perl behavior
- treat real Perl oracle output as editor-runtime truth
- promote provider behavior, diagnostics, rename, safe delete, semantic tokens,
  workspace symbols, or support tiers
- imply full CPAN cleanliness or full static Perl support

## Required Receipt Fields

Future implementations may choose exact Rust and JSON names, but the v1 receipt
must expose these semantic fields:

```text
schema_version
surface
workspace_or_fixture_identity
source_snapshot
source_hash_mode
toolchain_identity
compiler_model_versions
configuration_inputs
module_path_authority
ambient_inputs
generated_inputs
dynamic_boundaries
stale_or_partial_state
cache_or_index_state
provider_behavior_changed
determinism_state
fallback
blockers
unknowns
proof_commands
user_message
copyable_payload
```

`determinism_state` must be one of:

```text
repeatable
bounded_with_ambient_inputs
bounded_with_dynamic_boundaries
stale_or_partial
unsupported
unknown
```

`repeatable` is allowed only when the receipt can name all inputs required to
repeat the proof. If any ambient input, dynamic boundary, stale index, partial
workspace, generated/no-source fact, cache state, or unknown source affects the
result, the state must be bounded, stale/partial, unsupported, or unknown.

## Input Classification

Every input in a determinism receipt must map to the provenance and ambient
contracts:

| Input class | Examples | Determinism handling |
| --- | --- | --- |
| SourceBacked | workspace file range, lexical `use lib`, modeled FindBin path | May support repeatable proof when source snapshot is declared. |
| WorkspaceConfig | configured include paths, configured Perl binary, explicit LSP settings | Repeatable only when the config identity is declared. |
| ProcessEnvironment | `PERL5LIB`, `PERL5OPT`, local::lib variables | Ambient; must be listed or denied by the seam. |
| InterpreterState | startup `@INC`, Perl version, core roots | Oracle-derived ambient input; must not become workspace source. |
| ClientRuntimeState | DAP launch metadata, perldoc state, editor client capabilities | Report metadata unless a separate behavior receipt grants authority. |
| GeneratedInput | generated roots, build outputs, framework-generated members | Must distinguish source-backed generated from generated/no-source. |
| CacheOrIndexState | workspace index, open-document overlay, retained facts | Must report freshness and partial/stale state. |
| ExternalOracle | real Perl, perldoc, CPAN installation | Differential/helper oracle only, never editor-runtime truth by default. |
| Unknown | unclassified input or unmodeled runtime effect | Blocks repeatable proof. |

## Valid PR Shapes

Valid PRs under this spec include:

- docs PRs that define determinism receipt fields and claim boundaries
- schema PRs that add `determinism_receipt.v1` without provider behavior
- validator PRs that check receipt shape, links, redaction, or required fields
- fixture PRs that emit a receipt from already-declared test inputs
- PIR receipt PRs that explain lowering determinism without claiming runtime
  determinism
- workspace trust or module-resolution PRs that report determinism input classes
  without changing resolver behavior

Every valid PR must say whether it changes docs, schemas, validators, fixture
receipts, PIR lowering, workspace trust rendering, module resolution behavior,
provider behavior, subprocess seams, or support claims.

## Invalid PR Shapes

Invalid PRs include:

- adding receipt fields that imply full static Perl support
- claiming deterministic behavior from PIR node coverage alone
- claiming provider correctness from repeatable fixture output alone
- promoting diagnostics, completion, navigation, semantic tokens, rename, or
  safe-delete from determinism plumbing alone
- silently trusting ambient `PERL5LIB`, startup `@INC`, DAP metadata, perldoc, or
  real Perl oracle output as workspace source
- running Perl, `perldoc`, DAP, or workspace scans from explanation-only or
  report-only commands
- exposing raw secrets, private environment values, unnecessary raw paths, or
  launch payloads
- bundling determinism receipt work with unrelated parser, provider, refactor,
  release, or source-sync changes

## Acceptance

A PR satisfies this spec when:

- the touched receipt, schema, or docs name the determinism surface
- every touched input class has authority and provenance
- ambient and oracle inputs remain labeled and bounded
- generated/no-source and dynamic inputs cannot become repeatable source proof
- stale, partial, cache, and open-document states are visible
- provider behavior remains unchanged unless a separate promotion row and receipt
  explicitly allow it
- support-tier wording does not outgrow current receipts
- user-facing text explains what is repeatable, bounded, unsupported, or unknown

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

Schema or validator PRs must also run focused schema/validator tests. Runtime,
PIR, workspace trust, module-resolution, provider, or subprocess PRs must also
run the focused tests for the changed surface.

## Non-goals

- No determinism receipt generator from this spec alone.
- No PIR implementation.
- No retained cache contract.
- No runtime probing.
- No `perldoc`, DAP, real Perl, or application execution.
- No provider behavior change.
- No support-tier promotion.
- No release-lineage sync.
- No full CPAN, full static Perl, or deterministic Perl-language claim.

## Claim Boundaries

This spec may claim that determinism receipts must name inputs, boundaries,
freshness, cache/index state, fallback, blockers, and unknowns. It may not claim
that determinism receipts exist, that PIR is implemented, that provider behavior
is deterministic, that runtime Perl behavior is modeled, or that any support
tier has been promoted.
