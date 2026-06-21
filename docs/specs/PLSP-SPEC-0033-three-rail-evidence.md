# PLSP-SPEC-0033: Three-rail evidence contract

Status: draft
Owner: perl-lsp maintainers
Linked proposal: [PLSP-PROP-0001](../proposals/PLSP-PROP-0001-real-perl-editor-trust.md)
Linked program: compiler program — canonical current-main roadmap ([#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559), authored in parallel)
Linked boundary ADR: HIR body / PIR-A / EIR boundary ADR ([#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564), authored in parallel)
Linked specs:
- [PLSP-SPEC-0025](PLSP-SPEC-0025-pir-v0.md)
- [PLSP-SPEC-0026](PLSP-SPEC-0026-determinism-receipt-v1.md)
- [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md)
- [PLSP-SPEC-0030](PLSP-SPEC-0030-compile-state-layers.md)
- [PLSP-SPEC-0032](PLSP-SPEC-0032-pir-a-places-effects-cfg.md)
Linked issues:
- [#2559](https://github.com/EffortlessMetrics/perl-lsp/issues/2559) — compiler program tracker
- [#2563](https://github.com/EffortlessMetrics/perl-lsp/issues/2563) — context model
- [#2564](https://github.com/EffortlessMetrics/perl-lsp/issues/2564) — boundary ADR
- [#2565](https://github.com/EffortlessMetrics/perl-lsp/issues/2565) — two-layer IR (PIR-A vs EIR)
Linked roadmap: [Compiler-backed LSP roadmap](../project/COMPILER_BACKED_LSP_ROADMAP.md)
Status impact: compiler fact substrate evidence, determinism receipts, oracle
receipts, fixture corpus, fact-class KPIs

## Purpose

The compiler substrate must prove three different properties, and each requires a
different kind of evidence. Conflating them produces circular or hollow proof.
This spec fixes a **three-rail evidence model** so reviewers can tell which rail
a given fixture or receipt belongs to, and what it is and is not allowed to
prove.

The three rails are independent and complementary:

| Rail | Proves | Source of truth | Builds on |
| --- | --- | --- | --- |
| **Snapshot** | stability (output did not change unexpectedly) | the committed prior output | — |
| **Curated-gold** | correctness (output matches independent human labeling) | a human who labeled the expected facts | — |
| **Differential oracle** | real-Perl agreement (bounded conformance) | real Perl on declared fixtures | [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md) |

## Contract

### C1 — Snapshot rail proves stability only

A snapshot fixture captures current output and fails when output changes. It
proves the system is **stable**, not that it is **correct**: a snapshot of a
wrong answer is a stably wrong answer.

Naming: snapshot fixtures and tests use the term **snapshot** (consistent with
the existing `*.snap` insta snapshots and `tests/snapshots/` directories in the
repo). They must **never** be named "gold" or implied to be correctness proof.
"Gold" is reserved for the curated-gold rail (C2). A snapshot update is a stability
event, reviewed for intent; it is not evidence of correctness.

### C2 — Curated-gold rail proves correctness

A curated-gold fixture carries an **independently human-labeled** expected
result for a fact class — the expected packages, scopes, places, contexts,
effects, or boundaries — written by a person reasoning about Perl semantics, not
copied from the system's own output.

- The labeling must be independent of the lowering pipeline. A gold expectation
  generated from `lower_ast()`, from PIR lowering, or from any other path under
  test is **circular** and is not valid gold.
- The initial curated-gold set starts at **30–50 dense fixtures** — small,
  semantically rich cases (context propagation, place evaluation-once, `local`
  save/restore, `&&`/`||`/`//` last-value, signature binding, dynamic
  boundaries) — not a broad shallow sweep. Density over breadth.
- Each gold fixture states the fact class it labels and the rationale for the
  expected value, so a reviewer can check the human reasoning, not just the
  diff.

### C3 — Differential oracle rail proves bounded real-Perl agreement

The differential oracle compares Rust facts against real Perl on **declared,
bounded** fixtures, governed entirely by
[PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md): hermetic
environment, declared fixtures, disagreement classification, and the rule that
real Perl is never an editor-runtime dependency. This rail proves agreement
within the executable profile's boundary
([PLSP-SPEC-0035](PLSP-SPEC-0035-executable-profile-and-eir.md)); it does not
prove correctness outside that boundary, and oracle agreement alone never
promotes provider behavior.

### C4 — Rails are not interchangeable

- Snapshot agreement does not imply correctness or real-Perl agreement.
- Gold agreement does not imply stability across refactors (that is the
  snapshot rail) nor real-Perl conformance (that is the oracle rail).
- Oracle agreement is bounded by the executable profile and does not imply
  correctness for unsupported constructs.

A promotion claim that needs correctness must cite the gold rail; a claim that
needs conformance must cite the oracle rail; a claim that needs "no unintended
change" cites the snapshot rail. No rail is substituted for another.

### C5 — Gold-from-output is circular

Any gold expectation derived from the artifact under test — `lower_ast()`, PIR/
PIR-A lowering, or a provider's own output — is circular and invalid as gold. It
may live in the snapshot rail (stability) but must not be labeled correctness
proof. CI and reviewers reject "gold" fixtures whose values were machine-copied
from the pipeline.

## KPIs by fact class

Evidence is tracked per fact class, not as a single aggregate, so a weak class
cannot hide behind a strong one. For each fact class
(PackageSubTable, Scope/Pad, Place, Context, Effect/CFG, Import/Export, ISA,
Constant/Prototype, DynamicBoundary, FrameworkGenerated), the substrate tracks:

- **snapshot coverage**: count of fact instances under a stability snapshot
- **gold coverage**: count of independently labeled gold fixtures and the
  fraction of the class they exercise
- **gold agreement**: fraction of gold expectations the system matches
- **oracle coverage / agreement**: per
  [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md) comparison
  classes and result kinds
- **unknown / dynamic-boundary rate**: fraction resolved to `Unknown` or a
  dynamic boundary (honest-uncertainty signal, not a failure)

KPIs are reported per class; an aggregate "X% correct" claim with no per-class
breakdown is out of contract.

## Valid PR Shapes

Valid PRs under this spec include:

- adding snapshot fixtures named as snapshots, for stability
- adding independently human-labeled gold fixtures with stated rationale, toward
  the initial 30–50 dense set
- adding oracle fixtures and receipts per
  [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md)
- adding per-fact-class KPI reporting
- documentation that keeps the three rails distinct

Every evidence PR must name which rail it adds to, the fact class, and (for gold)
the independent-labeling source.

## Invalid PR Shapes

Invalid PRs include:

- naming a snapshot "gold" or treating snapshot agreement as correctness
- generating gold expectations from `lower_ast()` or any artifact under test
  (the corrected circular-gold pattern, C5)
- reporting a single aggregate correctness number without per-class KPIs
- promoting provider behavior from snapshot or oracle agreement alone
- treating real Perl as an editor-runtime dependency (see
  [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md))

## Acceptance

A PR satisfies this spec when:

- each added fixture/receipt is assigned to exactly one rail with the right name
- gold fixtures are independently labeled with stated rationale and are not
  derived from the pipeline
- KPIs are reported per fact class
- no rail is cited to prove a property it does not establish

## Proof Commands

Docs-only changes to this spec may use:

```bash
cargo xtask ci-hygiene check-doc-paths docs/specs
cargo xtask ci-hygiene check-doc-paths docs/project/status
git diff --check
```

Implementation PRs add fixtures and KPI reporting and run the owning crate's
tests (for example `cargo test -p perl-parser-core --locked`) plus oracle checks
when the oracle rail is touched (see
[PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md) Proof
Commands).

## Non-goals

- No oracle runner from this spec alone (governed by
  [PLSP-SPEC-0027](PLSP-SPEC-0027-differential-real-perl-oracle.md)).
- No provider behavior change from evidence alone.
- No determinism claim beyond
  [PLSP-SPEC-0026](PLSP-SPEC-0026-determinism-receipt-v1.md).
- No real-Perl editor-runtime dependency.
- No correctness claim from snapshots.

## Claim Boundaries

This spec may claim that evidence is organized into three independent rails —
snapshot (stability), curated-gold (correctness), differential oracle (bounded
real-Perl agreement) — with per-class KPIs and a ban on circular gold. It may not
claim any rail's coverage is complete, that the substrate is correct, or that any
provider has been promoted, until the rail's own receipts and status rows make
that claim.
