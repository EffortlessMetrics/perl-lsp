# Measured Perl Editor Trust

`perl-lsp` is designed to be honest about what it can know statically in real
Perl projects. The goal is not to guess more often. The goal is to act when the
facts are strong, fall back when they are weak, and refuse edits when static
evidence cannot make the action safe.

The current user-facing claim map is
[SUPPORT_TIERS.md](../project/status/SUPPORT_TIERS.md). This page explains the
same trust model in plain language. If this page and a status document disagree,
the status document is the source of truth.

## What perl-lsp Measures

Parser compatibility is measured through generated parser status and parser
accuracy receipts:

- [parser_accuracy_next.md](../project/status/parser_accuracy_next.md) says
  whether parser work should close measurement gaps or move to capability
  buckets.
- [parser.md](../project/status/parser.md) records corpus baselines, raw failure
  buckets, and receipt freshness.

Raw parser buckets are discovery input. A stale bucket can justify a focused
source-backed fixture, but only a fresh corpus receipt can justify a bucket-count
claim. `perl-lsp` does not claim full CPAN cleanliness.

Provider behavior is tracked through:

- [provider_confidence_matrix.md](../project/status/provider_confidence_matrix.md)
- [provider_cutover.md](../project/status/provider_cutover.md)
- [semantic_scorecard.md](../project/status/semantic_scorecard.md)
- [semantic_shadow_compare.md](../project/status/semantic_shadow_compare.md)
- [real_perl_editor_trust_v1.md](../project/status/real_perl_editor_trust_v1.md)

Those docs record the fact source, confidence, freshness, fallback behavior, and
next proof for each editor surface.

## What Partial-Live Means

Some provider paths are live only for narrow, proven cases. That is what
`partial-live-with-fallback` means in the support map.

Examples:

- Completion may use high-confidence imported or exported facts, while generated
  or dynamic candidates stay shadowed or labeled.
- Goto definition and references may answer from one fresh, source-backed exact
  or imported candidate, while ambiguous or dynamic candidates fall back.
- Rename is live for the scoped same-file lexical case. Broader package,
  generated, dynamic, stale, or low-confidence rename plans remain blocked or
  shadowed.

Partial-live is intentionally narrower than "the feature is complete." It means
the editor can use the proven path without pretending every related Perl form is
equally safe.

## Why the Editor Falls Back

Fallback is a trust decision, not just a missing feature.

The editor may fall back when:

- no source-backed fact exists;
- multiple candidates are plausible;
- a fact is stale relative to the current workspace;
- the only candidate comes from generated or framework behavior;
- Perl runtime behavior makes the result dynamic;
- the provider has a receipt but has not been promoted to live behavior.

Fallback keeps existing behavior available while preventing a shadow receipt or
low-confidence candidate from becoming a false exact answer.

## Why Rename and Safe Delete May Refuse

Refactors can damage code, so their proof bar is higher than completion or hover.

Rename and safe delete should refuse or block when a requested edit depends on:

- stale compiler facts;
- low-confidence name matches;
- generated members without source ranges;
- dynamic Perl behavior;
- imported or exported symbols with dependent references;
- package-wide or workspace-wide plans without rollback proof.

A refusal is the correct result when `perl-lsp` cannot prove the edit is safe.
The current support map records which refactor classes are live, shadowed, or
blocked.

## How Dynamic Perl Affects Static Tooling

Perl commonly creates behavior at runtime. Static tooling cannot always turn
that into exact source locations.

`perl-lsp` treats these as boundaries instead of pretending they are ordinary
static facts:

- `AUTOLOAD` and generated methods;
- `eval STRING`;
- dynamic `require` or computed module names;
- typeglob manipulation;
- framework-generated accessors or routes;
- imports and exports that depend on runtime control flow.

When a provider sees a dynamic boundary, the correct result may be a label, a
fallback, or a refusal. It should not silently become an exact edit or source
jump unless a specific receipt proves that case.

## How Module Resolution and @INC Are Handled

Module resolution is tracked separately because it affects diagnostics,
completion, hover, goto definition, and DAP behavior.

The user-facing language-support reference is
[LANGUAGE_SUPPORT.md](../reference/LANGUAGE_SUPPORT.md). The current consistency
matrix is [module_resolution.md](../project/status/module_resolution.md).

The default model is conservative:

- workspace roots and configured include paths are explicit;
- lexical `use lib` and `no lib` are handled when statically visible;
- system `@INC` and `PERL5LIB` are controlled by configuration;
- dynamic `require` forms are treated as evidence boundaries, not guessed paths;
- DAP and Perl subprocess behavior is tracked through explicit
  `PerlOracleEnv` contracts.

## What Real-Workspace Receipts Prove

Synthetic fixtures prove focused behavior. Real-workspace receipts prove that
the behavior still holds in project-shaped Perl.

Current real-workspace receipts include Mojolicious and Dancer2 baselines linked
from the [Real Perl Editor Trust dashboard](../project/status/real_perl_editor_trust_v1.md).
They cover named projects on named hosts. They do not prove all CPAN projects,
all platforms, or broad live provider cutover.

Real-workspace proof can cover:

- cold start and indexing;
- module resolution;
- completion and navigation quality;
- hover and diagnostics behavior;
- symbol and semantic-token surfaces;
- rename or safe-delete blockers;
- freshness, fallback, and dynamic-boundary behavior.

Support claims should widen only when the corresponding receipt covers the
claim being made.

## How To Report a Wrong Result

Useful bug reports include the trust boundary, not only the visible symptom.

When reporting completion, goto, references, hover, diagnostics, rename, or safe
delete behavior, include:

- the provider surface;
- whether the result acted, fell back, or refused;
- the file and cursor location;
- the relevant module path or `@INC` configuration;
- whether the code uses dynamic Perl features;
- the expected safe behavior.

`perl.explainProviderDecision` is the command surface for structured provider
decision explanations. The current v1 command is conservative: when no
provider-specific receipt is attached, it returns a low-confidence
`missing_fact` / `no_result` fallback rather than inventing certainty. Future
provider wiring can attach live receipts to that same shape.

## Where To Check Current Claims

Use these docs when you need the current support boundary:

- [SUPPORT_TIERS.md](../project/status/SUPPORT_TIERS.md) for user-facing claims,
  proof commands, limitations, and next promotion proof.
- [provider_confidence_matrix.md](../project/status/provider_confidence_matrix.md)
  for provider fact source, confidence, freshness, fallback, and next proof.
- [real_perl_editor_trust_v1.md](../project/status/real_perl_editor_trust_v1.md)
  for the current trust-lane routing dashboard.
- [parser_accuracy_next.md](../project/status/parser_accuracy_next.md) and
  [parser.md](../project/status/parser.md) for parser measurement and bucket
  routing.

