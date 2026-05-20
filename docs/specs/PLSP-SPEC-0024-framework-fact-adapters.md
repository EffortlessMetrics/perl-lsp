# PLSP-SPEC-0024: Framework fact adapter contract

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0016](PLSP-SPEC-0016-provider-decision-receipt-v1.md)
- [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- [PLSP-SPEC-0018](PLSP-SPEC-0018-edit-authorization-contract.md)
- [PLSP-SPEC-0020](PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Policy: [framework-adapters.toml](../../policy/framework-adapters.toml)
Status impact: generated-member facts, provider confidence matrix, provider
promotion ledger, support tiers, workspace-symbol generated-label pilot,
rename and safe-delete blockers

## Current Implementation Status

The compiler substrate already has fixture-backed generated-member facts and
provider receipts that label selected generated/framework, dynamic-boundary, and
fallback paths. Workspace symbols have a bounded generated-label pilot, and
rename/safe delete already block generated members unless a separate proof
authorizes the edit.

This spec defines the contract for future framework adapters. It does not add a
framework adapter, broaden generated-symbol behavior, promote exact generated
definitions, or authorize generated-member edits.

## Contract

Framework support must emit facts into the shared compiler/provenance model. It
must not become provider-local special casing.

Every framework adapter fact must identify:

- the framework kind
- the generated fact kind
- the generated symbol or relationship name
- the source declaration anchor, when one exists
- confidence and freshness
- provenance under [PLSP-SPEC-0017](PLSP-SPEC-0017-fact-provenance-and-source-backing.md)
- dynamic-boundary reason when the adapter cannot model the behavior
- rename policy
- safe-delete policy
- workspace-symbol label text, when the fact may surface there
- hover provenance text, when the fact may surface there

Generated framework facts are source-backed only when the adapter can point to a
workspace source declaration that creates the virtual member or relationship.
Runtime-only, heuristic, stale, partial, or unanchored framework evidence is
`GeneratedNoSource`, `DynamicBoundary`, `AmbientInput`, or `Unknown`, and must
not become exact provider behavior.

## Target Code Shape

Future code may introduce a common type with this shape:

```rust
pub struct FrameworkGeneratedFact {
    pub framework: FrameworkKind,
    pub fact_kind: GeneratedFactKind,
    pub generated_name: SymbolName,
    pub declaration_range: Option<SourceRange>,
    pub confidence: Confidence,
    pub provenance: FactProvenance,
    pub rename_policy: RefactorPolicy,
    pub safe_delete_policy: RefactorPolicy,
}
```

Until that type exists, existing generated-member records, framework adapter
helpers, provider receipts, workspace-symbol labels, hover provenance payloads,
rename blockers, and safe-delete blockers must preserve the same semantics.

## Adapter Fact Classes

### GeneratedMember

`GeneratedMember` facts model source-declared virtual members such as accessors,
predicates, clearers, writers, constructors, route symbols, and delegated
methods.

Allowed behavior:

- may appear in completion or hover as generated/framework provenance
- may appear in workspace symbols only under
  [PLSP-SPEC-0020](PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md)
- may block rename and safe delete by default
- may feed determinism or oracle receipts as modeled compile facts

Forbidden behavior:

- must not claim exact generated method-body locations
- must not authorize rename or safe delete without class-specific edit proof
- must not hide the generated/framework label when shown to users
- must not be treated as an explicit source fact

### Inheritance

`Inheritance` facts model static `extends`, `with`, `use base`, or equivalent
framework declarations when the parent or role names are literal and source
anchored.

Allowed behavior:

- may improve receiver, hover, completion, definition, and reference receipts
  inside their support tier
- may contribute to determinism and oracle receipts
- may explain fallback when inheritance is dynamic or unmodeled

Forbidden behavior:

- must not resolve dynamic parent or role names as exact facts
- must not authorize edits across inherited methods without separate reference
  and rollback proof

### RouteSymbol

`RouteSymbol` facts model static route declarations from web frameworks when the
route name, handler, or controller binding has a source anchor.

Allowed behavior:

- may appear as labeled virtual project shape
- may explain hover, workspace symbols, and diagnostics where receipts exist
- may report dynamic route construction as a boundary

Forbidden behavior:

- must not infer runtime route tables from execution
- must not scan or run the application
- must not become exact handler navigation without source-backed handler proof

### ImportFact

`ImportFact` facts model framework-provided imports and exports only when the
adapter can identify the imported name, provider package, and source declaration
or configured framework contract.

Allowed behavior:

- may explain symbol visibility
- may improve completion, hover, definition, references, and diagnostics inside
  the provider support tier
- may block rename and safe delete when import/export ambiguity exists

Forbidden behavior:

- must not treat ambient module path state as workspace source
- must not silently suppress diagnostics from weak or stale import evidence

### DynamicBoundary

`DynamicBoundary` facts model framework behavior that depends on runtime code
loading, `AUTOLOAD`, symbolic references, typeglob mutation, dynamic route
construction, string `eval`, dynamic `require`, unsupported `BEGIN`, or
unmodeled metaclass behavior.

Allowed behavior:

- may explain fallback and blocker decisions
- must block edit-producing behavior
- may feed determinism receipts

Forbidden behavior:

- must not become exact completion, navigation, symbol, token, rename, or delete
  proof

## First Adapter Order

The advisory adapter registry starts with:

- `Class::Tiny`
- simple `Moo has`
- simple `Moose has`, `extends`, and `with`
- `AUTOLOAD` boundary
- static `DBIx::Class` subset
- static `Catalyst` and `Dancer` route symbols
- selected `Test::More` and `Try::Tiny` imports
- `Object::Pad` classes and fields

Unlisted adapters are blocked or deferred until a policy row and proof PR define
their fact class, source-anchor rule, fallback rule, blocker rule, and receipt.

## Policy Registry

The advisory registry in
[framework-adapters.toml](../../policy/framework-adapters.toml) records planned,
pilot, blocked, and deferred adapter classes. It does not broaden provider
behavior. Provider promotion remains controlled by
[provider-promotion-ledger.toml](../../policy/provider-promotion-ledger.toml).

## Valid PR Shapes

Valid PRs under this spec include:

- adding one adapter fact class with source-anchor, provenance, fallback, and
  blocker rules
- adding one adapter policy row without behavior changes
- adding provider receipts for one adapter class
- adding generated/no-source or dynamic-boundary blocker receipts
- adding hover, workspace-symbol, rename, or safe-delete tests for one adapter
  class
- adding an adapter registry validator

Every adapter PR must name one fact class, one provider surface, one promotion
rule, one fallback rule, one blocker rule, and one receipt.

## Invalid PR Shapes

Invalid PRs include:

- broad framework support claims from one adapter proof
- provider-local framework hacks that bypass fact provenance
- generated/no-source facts appearing as exact symbols or definitions
- unlabeled generated workspace symbols
- generated-member rename or safe-delete without class-specific edit proof
- dynamic framework behavior treated as source-backed exact proof
- running Perl, perldoc, DAP, or application code to discover adapter facts
- support-tier promotion without current receipts and ledger rows

## Acceptance

A framework adapter PR satisfies this spec when:

- the adapter emits only the named fact class
- source-backed generated facts have source declaration anchors
- generated/no-source and dynamic candidates are blocked or explanation-only
- rename and safe-delete policies are explicit
- provider decisions expose generated, fallback, or blocker state
- workspace symbols preserve generated labels and virtual source-anchor semantics
- hover provenance names the framework and confidence
- support-tier wording remains conservative unless a support-review PR promotes
  the class

## Proof Commands

Docs-only changes to this spec or policy may use:

```bash
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
cargo xtask check-provider-promotion-ledger
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Adapter behavior PRs must add or update focused receipts for the touched
framework class and provider surface.

Future framework-adapter policy changes should also run a dedicated
`cargo xtask check-framework-adapters` validator once it exists.

## Non-goals

- No provider behavior change from this spec alone.
- No broad framework support claim.
- No broad generated-member cutover.
- No generated/no-source exact behavior.
- No generated-member rename or safe-delete authorization.
- No Perl execution, `perldoc` execution, DAP launch, or application probing.
- No support-tier promotion from this spec alone.

## Claim Boundaries

This spec may claim that framework adapters are governed by shared provenance,
labeling, blocker, and refactor-policy rules. It may not claim that any
framework adapter is implemented, broadly supported, exact, edit-safe, or
runtime-complete unless separate receipts, support tiers, and promotion-ledger
rows make that claim.
