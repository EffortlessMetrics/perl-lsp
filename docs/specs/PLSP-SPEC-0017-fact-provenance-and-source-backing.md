# PLSP-SPEC-0017: Fact provenance and source backing

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked specs:
- [PLSP-SPEC-0002](PLSP-SPEC-0002-provider-confidence-receipts.md)
- [PLSP-SPEC-0008](PLSP-SPEC-0008-edit-producing-provider-safety.md)
- [PLSP-SPEC-0015](PLSP-SPEC-0015-real-perl-editor-trust-v1-boundary.md)
- [PLSP-SPEC-0016](PLSP-SPEC-0016-provider-decision-receipt-v1.md)
- [PLSP-SPEC-0018](PLSP-SPEC-0018-edit-authorization-contract.md)
- [PLSP-SPEC-0020](PLSP-SPEC-0020-workspace-symbol-generated-label-contract.md)
- [PLSP-SPEC-0022](PLSP-SPEC-0022-module-path-authority.md)
- [PLSP-SPEC-0023](PLSP-SPEC-0023-ambient-inputs.md)
Linked ADRs:
- [PLSP-ADR-0002](../adr/PLSP-ADR-0002-confidence-before-cutover.md)
Linked plan: [Real Perl Editor Trust implementation plan](../../plans/real-perl-editor-trust/implementation-plan.md)
Status impact: provider confidence matrix, provider promotion ledger, support
tiers, Real Perl Editor Trust dashboard, provider decision receipts

## Current Implementation Status

The current implementation already carries provenance through
`perl_semantic_facts::Provenance`, `ProviderFactSourceKind`, provider-local
receipts, generated-member facts, dynamic-boundary facts, and source anchors.
Those types are existing projections of this contract.

This spec defines the shared ontology that future providers, framework
adapters, edit authorization, determinism receipts, and provider explanations
must use. It does not require a Rust type rename or broad provider cutover in
the PR that adds this spec.

## Contract

A fact can help the user only inside its proof boundary.

Every provider-facing fact must be classifiable as one of:

```text
ExplicitSource
SourceBackedGenerated
GeneratedNoSource
DynamicBoundary
AmbientInput
Unknown
```

The classification determines what the fact may do:

- drive exact provider behavior
- appear as a labeled virtual/generated symbol
- explain a fallback or blocker
- report ambient setup state
- block edit-producing behavior

No provider may silently treat generated/no-source, dynamic, ambient, unknown,
stale, low-confidence, or ambiguous evidence as exact source-backed proof.

## Target Code Shape

Future code may introduce a common type with this shape:

```rust
pub enum FactProvenance {
    ExplicitSource {
        file: FileId,
        range: SourceRange,
    },
    SourceBackedGenerated {
        framework: FrameworkKind,
        declaration_range: SourceRange,
        generated_name: SymbolName,
    },
    GeneratedNoSource {
        framework: Option<FrameworkKind>,
        reason: GeneratedNoSourceReason,
    },
    DynamicBoundary {
        boundary: DynamicBoundaryKind,
        range: Option<SourceRange>,
    },
    AmbientInput {
        input: AmbientInputKind,
    },
    Unknown,
}
```

Until that type exists, existing `Provenance`, `ProviderFactSourceKind`,
`EntityKind`, `OccurrenceKind`, generated-member records, provider receipts,
and workspace trust report fields must preserve the same semantics.

## Provenance Classes

### ExplicitSource

`ExplicitSource` means the fact is anchored directly to workspace source text.
The anchor must identify a file and a source range.

Examples:

- package declaration parsed from a source file
- subroutine declaration parsed from a source file
- lexical declaration in the current document
- explicit import or export declaration with a known anchor
- source-backed reference occurrence

Allowed behavior:

- may drive exact provider behavior when confidence, freshness, and
  provider-specific guards pass
- may authorize edit-producing behavior only when edit-authorization guards
  also pass
- may appear in completion, hover, definition, references, symbols, tokens, and
  diagnostics inside the support tier boundary

Required blockers:

- stale fact
- low confidence
- ambiguous identity
- missing source range
- unsupported fact class
- provider-specific unsafe-edit guard failure

### SourceBackedGenerated

`SourceBackedGenerated` means a framework declaration in source generates a
virtual symbol or relationship. The generated member has a source-backed
declaration anchor, but it does not have an exact generated method body range.

Examples:

- Moo or Moose `has name => (...)` generating an accessor
- Class::Tiny attribute declaration generating accessors
- route declaration generating a route symbol
- framework adapter fact with a source declaration range

Allowed behavior:

- may appear as a labeled virtual/generated symbol
- may explain hover and completion provenance
- may participate in workspace symbols only under the generated-label contract
- may block rename and safe delete by default

Forbidden behavior:

- must not claim an exact generated method-body location
- must not authorize rename or safe-delete unless a separate class-specific
  edit proof exists
- must not be presented as `ExplicitSource`

Required label:

```text
<name> - generated by <Framework> from <declaration>
```

### GeneratedNoSource

`GeneratedNoSource` means a provider has evidence of a generated, runtime, or
framework-shaped candidate without a source declaration anchor.

Examples:

- method inferred from runtime framework behavior without a source declaration
- generated member known by naming convention only
- generated symbol from partial or stale framework evidence
- no-source generated candidate from external or runtime-only metadata

Allowed behavior:

- may appear in receipts as blocked, deferred, or explanation-only evidence
- may explain why a stronger claim is unavailable

Forbidden behavior:

- never exact
- never edit-authorizing
- never a generated workspace-symbol live result
- never an exact definition target

### DynamicBoundary

`DynamicBoundary` means the relevant Perl behavior depends on dynamic execution
or runtime mutation that the compiler substrate has not modeled.

Examples:

- string `eval`
- dynamic `require`
- symbolic references
- typeglob aliasing
- `AUTOLOAD`
- dynamic method name
- dynamic bless class
- dynamic hash key used as exact receiver evidence
- unsupported `BEGIN` behavior

Allowed behavior:

- may explain fallback
- may appear as a diagnostic or provider-decision blocker
- may report a determinism boundary

Forbidden behavior:

- must block edit-producing behavior
- must not become exact definition, reference, symbol, token, or receiver proof
- must not suppress diagnostics as if the symbol were known source-backed

### AmbientInput

`AmbientInput` means the fact depends on state outside workspace source.

Examples:

- configured include paths
- `PERL5LIB`
- system `@INC`
- generated or `blib` roots
- launch `env.PERL5LIB`
- client-supplied DAP include path metadata
- perldoc or real-Perl oracle boundary

Allowed behavior:

- may be reported by workspace trust and determinism receipts
- may explain module-resolution context
- may influence behavior only when a spec names that authority

Forbidden behavior:

- must not be silently treated as workspace source
- must not imply `@INC` authority from report-only DAP metadata
- must not run probes or subprocesses from explanation-only surfaces

### Unknown

`Unknown` means the system lacks enough classified evidence to make a stronger
claim.

Allowed behavior:

- fallback
- block
- defer
- explanation-only

Forbidden behavior:

- no exact provider behavior
- no edit authorization
- no support-tier promotion

## Current Projection Mapping

Existing code should be interpreted through this mapping until a first-class
`FactProvenance` type exists:

| Current signal | Fact-provenance class |
|---|---|
| `Provenance::ExactAst` | `ExplicitSource` |
| `Provenance::DesugaredAst` | `ExplicitSource` when the desugared range points back to source |
| `Provenance::SemanticAnalyzer` | `ExplicitSource` when it carries a source anchor; otherwise `Unknown` |
| `Provenance::ImportExportInference` | `ExplicitSource` when import/export anchors exist |
| `Provenance::LiteralRequireImport` | `ExplicitSource` when require/import anchors exist |
| `Provenance::PragmaInference` | `ExplicitSource` when pragma source anchors exist |
| `Provenance::FrameworkSynthesis` with source anchor | `SourceBackedGenerated` |
| `Provenance::FrameworkSynthesis` without source anchor | `GeneratedNoSource` |
| `Provenance::NameHeuristic` | `Unknown` or `GeneratedNoSource`; never exact by itself |
| `Provenance::SearchFallback` | `Unknown` or fallback evidence |
| `Provenance::DynamicBoundary` | `DynamicBoundary` |
| `ProviderFactSourceKind::FrameworkAdapter` | `SourceBackedGenerated` only when declaration anchor exists; otherwise `GeneratedNoSource` |
| `ProviderFactSourceKind::DynamicBoundary` | `DynamicBoundary` |
| `VisibleSymbolSource::Generated` | `SourceBackedGenerated` only when declaration anchor exists; otherwise `GeneratedNoSource` |
| `VisibleSymbolSource::DynamicUnknown` | `DynamicBoundary` or `Unknown` |
| workspace trust setup state | `AmbientInput` |

When a current signal could map to more than one class, the provider must choose
the more conservative class unless a receipt proves the stronger one.

## Provider Rules

Completion, hover, definition, references, diagnostics, document symbols,
workspace symbols, semantic tokens, rename, safe delete, provider explanations,
and workspace trust report must follow the same provenance contract.

Required provider behavior:

- source-backed facts can drive exact behavior only inside their support tier
- generated facts must be labeled
- generated/no-source facts must be blocked, deferred, or explanation-only
- dynamic facts must block unsafe edits
- ambient inputs must be reported with authority labels
- stale or low-confidence facts must not authorize edits
- receipts must explain the selected decision
- support claims must follow receipts

## Valid PR Shapes

Valid PRs under this spec include:

- adding a first-class `FactProvenance` type
- mapping existing `Provenance` and provider fact-source values into the shared
  ontology
- adding provider receipts that prove a generated fact is source-backed
- adding blockers for generated/no-source or dynamic candidates
- adding validators that require provenance, source backing, and blocker state
- docs PRs that clarify provenance without changing provider behavior

## Invalid PR Shapes

Invalid PRs include:

- promoting generated/no-source facts to exact behavior
- treating framework-generated members as exact method-body locations
- authorizing rename or safe-delete from generated, dynamic, ambient, unknown,
  stale, low-confidence, or ambiguous evidence
- treating `PERL5LIB`, system `@INC`, DAP include paths, perldoc, or real Perl
  oracle output as workspace source without a spec-defined authority
- suppressing diagnostics from low-confidence or dynamic facts as if they were
  source-backed
- broad provider cutover from provenance plumbing alone

## Acceptance

A PR satisfies this spec when:

- every provider-facing fact touched by the PR has a provenance class or
  explicitly remains unknown
- stronger source-backed claims require source range, confidence, and freshness
- generated facts are labeled or blocked
- generated/no-source facts do not produce exact locations or edits
- dynamic boundaries block unsafe edit-producing behavior
- ambient inputs are reported rather than silently trusted
- provider decision receipts expose the fallback or blocker boundary
- support-tier wording does not claim more than the provenance class supports

## Proof Commands

Docs-only PRs for this spec may use:

```bash
cargo xtask check-provider-confidence-matrix
cargo xtask check-support-claims
cargo xtask check-provider-promotion-ledger
cargo xtask ci-hygiene check-doc-paths docs/specs
git diff --check
```

Code PRs that add a provenance type or provider mapping must add focused unit
tests for the changed projection and run the affected provider or semantic
crate checks.

## Non-goals

- No provider behavior change from this spec alone.
- No broad generated workspace-symbol promotion.
- No broad semantic-token promotion.
- No rename or safe-delete authorization change.
- No determinism receipt implementation.
- No real-Perl oracle or subprocess dependency.
- No replacement for provider decision receipt v1.

## Claim Boundaries

This spec may be cited to block or defer provider behavior when provenance is
too weak. It may not be cited as proof that a provider is live or that a
generated, dynamic, ambient, or unknown fact is exact.
