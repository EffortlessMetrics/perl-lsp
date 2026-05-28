# Semantic Inline Completion Roadmap

Status: active implementation
Owner: perl-lsp maintainers
Related:
- [Inline Completion Release Gate](INLINE_COMPLETION_RELEASE_GATE.md)
- [LSP 3.18 conformance boundary](../specs/PLSP-SPEC-0029-lsp-318-conformance-boundary.md)
- [LSP implementation guide](../reference/LSP_IMPLEMENTATION_GUIDE.md)

## Purpose

This roadmap defines the lane that moves inline completion from "the method
exists" to a trusted Perl editing primitive.

The end state is:

```text
perl-lsp understands the Perl already on screen well enough to offer quiet,
correct, context-aware next text.
```

Inline completion must not become a snippet engine, an AI demo, or a fragile
LSP novelty. The useful version is narrower and harder: when a user is in a Perl
file, in their project, with their imports, lexicals, test framework, object
style, and editor, ghost text is rarely wrong, rarely annoying, and often the
next thing they were about to type.

Every suggestion must pass three product tests:

- it is syntactically safe at the cursor;
- it uses symbols, imports, and style already present or proven reachable;
- it would not be annoying if shown automatically.

## Current Boundary

The release gate already proves the embedded `perllsp` binary can act as a
standard LSP 3.18 `textDocument/inlineCompletion` target:

- static clients receive top-level `inlineCompletionProvider`;
- LSP4IJ-style dynamic clients receive `client/registerCapability` for
  `textDocument/inlineCompletion`;
- disabled `lsp.inline_completion` suppresses advertisement, registration, and
  runtime execution;
- a simple `use ` fixture returns deterministic `strict;`;
- neutral positions return empty.

Those receipts are protocol and process-boundary proof. The semantic lane has
since moved beyond the release smoke, but the claim boundary remains narrow:
implemented behavior is only trusted where covered by provider tests, stdio
smokes, and the deterministic quality receipt.

Current implemented foundations include:

- editor-safety contracts for replacement ranges, UTF-16 wire positions, and
  `selectedCompletionInfo` alignment;
- hard-zone suppression for comments, strings, heredocs, POD, regex bodies, and
  other unsupported syntax contexts covered by fixtures;
- an internal `SemanticInlineContext` used by deterministic candidate sources;
- candidate sources for syntax continuations, visible lexicals, workspace module
  imports, Test::More/Test2 assertions, current-package receiver methods, DBI
  receiver hints, constructor style, and contextual fallbacks;
- scored ranking that prefers local symbols, project modules, file role, and
  style-compatible candidates;
- a parse-safety filter that suppresses returned candidates which worsen local
  parser damage;
- local deterministic quality receipts that record fixture totals, source
  outcomes, latency, hard-zone suppression, replacement-range checks, and parse
  regressions.

The remaining work is not "make inline completion exist." It is to grow the
fixture corpus, keep the receipt counters honest, add real-editor/project-shape
receipts where useful, and only then consider next-edit or optional AI surfaces.

## Non-Goals

This lane must not become:

- AI-first completion;
- broad snippet expansion;
- a large provider rewrite without preserving behavior;
- broad LSP 3.18 conformance work;
- release plumbing;
- editor setup docs churn;
- general code-action work.

Optional AI belongs after deterministic candidates are parse-safe, ranked,
tested, and measured. AI candidates must remain off by default, explicitly
configured, cancellation-aware, timeout-bounded, and subject to the same range
and parse-safety filters as deterministic candidates.

## Product Contract

Inline completion should be conservative by default:

- automatic trigger: one high-confidence result or empty;
- invoked trigger: richer candidate set is allowed;
- selected completion popup: ghost text must extend the selected completion item
  or stay silent;
- unsupported zones: comments, strings, heredocs, POD, and unsupported ambiguous
  syntax should return empty;
- uncertain context: return empty instead of guessing.

Silence is a feature when the context is unsafe.

## Target Pipeline

The long-term deterministic pipeline is:

```text
request
  -> protocol and feature guard
  -> SemanticInlineContext
  -> candidate sources
  -> ranking
  -> parse-safety filter
  -> editor-safe response
  -> local/dev receipt counters
```

The pipeline must keep protocol concerns separate from candidate quality:

- protocol gates decide whether the request is legal and enabled;
- context extraction decides what Perl facts are visible;
- candidate sources propose possible next text;
- ranking decides what is worth showing;
- parse-safety and editor-safety filters decide what must be suppressed.

## Editor-Safety Contracts

### Replacement Ranges

Returned ranges must be editor-safe:

- start at the current token or partial expression;
- stay single-line unless a later explicit multiline contract is added;
- replace the typed prefix rather than duplicating it;
- use LSP UTF-16 positions on the wire;
- avoid fighting text typed after the request started.

Bad ranges make ghost text feel like an editor bug. Range correctness should
land before semantic expansion.

### `selectedCompletionInfo`

When `selectedCompletionInfo` is present, an inline item is valid only when it
uses a compatible range and extends the selected completion item. Conflicting
items must be suppressed.

This contract keeps ghost text from fighting an open completion popup.

### Hard Reject Zones

The provider must stay silent in zones where code ghost text is likely wrong:

- comments;
- string literals;
- heredoc bodies;
- POD;
- regex bodies unless a future PR adds explicit regex-context support.

## Semantic Context Target

`SemanticInlineContext` is the bridge from line scanning to project-aware Perl
suggestions. The current implementation keeps this context internal and narrow:

```rust
pub struct SemanticInlineContext {
    pub lexical_scope: ScopeId,
    pub package: Option<PackageId>,
    pub enclosing_sub: Option<SubId>,
    pub expected_syntax: ExpectedSyntax,
    pub visible_lexicals: Vec<VariableFact>,
    pub receiver_type_hint: Option<ReceiverHint>,
    pub imported_modules: Vec<ModuleFact>,
    pub file_role: FileRole,
}
```

Some fields still use best-effort facts when richer parser, semantic, or
workspace data is not available. The important contract is the direction:
candidate sources should consume semantic context instead of scraping only the
current line.

## Candidate Sources

The deterministic engine is split into candidate sources and should stay that
way as new suggestions are added:

```rust
trait InlineCandidateSource {
    fn candidates(&self, ctx: &SemanticInlineContext) -> Vec<InlineCandidate>;
}
```

Current and intended sources:

- `SyntaxSource`: safe Perl continuations such as `use`, `return`, `for`, and control
  flow scaffolding;
- `LexicalSource`: visible lexical variables and nearby symbols;
- `ImportSource`: workspace modules from effective include context;
- `TestSource`: Test::More and Test2-aware assertions in `.t` files;
- `ReceiverSource`: current package methods for `$self->` and narrow receiver
  hints such as DBI handles;
- `StyleSource`: constructor idioms, signature style, indentation, and local
  assertion style.

Adding sources should not mean adding noisy suggestions. Each source must have
negative fixtures proving where it stays silent.

## Ranking

Candidate ordering uses scored confidence rather than fixed rule order.

Useful score components:

- context match;
- semantic confidence;
- project style match;
- recent symbol bonus;
- file role bonus;
- edit-distance or prefix-continuation bonus;
- risk penalty;
- annoyance penalty.

Ranking should favor visible lexicals, project modules, local idiom, and
parse-safe continuations. It should penalize placeholders, risky multiline text,
and suggestions that are valid Perl but unlikely to be welcome as automatic
ghost text.

## Parse-Safety Filter

Parse safety does not mean the whole document must become valid Perl after every
candidate. It means the candidate must not make the local edit state worse.

The filter:

- splices the candidate into the current line or replacement range;
- parses the probe using the available parser path;
- rejects candidates that increase local error score;
- allows candidates that improve an incomplete construct;
- records returned-item parse regressions in local/dev receipts.

This is the core trust mechanism for automatic suggestions.

## Fixture UX Corpus

Inline completion should be tested as editor UX, not only as provider units.
The current fixture receipt is implemented as a Rust xtask fixture list; a future
YAML or data-file corpus is still acceptable if it improves reviewability.
Fixtures should encode expected and forbidden ghost text:

```yaml
source: |
  use Test::More;

  my $got = compute();
  <CURSOR>
expected:
  - "is($got, $expected, '...');"
not_expected:
  - "done_testing();"
  - "return $got;"
```

Seed fixtures:

- `replacement_range_partial_token.yml`
- `replacement_range_method_arrow.yml`
- `replacement_range_use_prefix.yml`
- `utf16_partial_token.yml`
- `selected_completion_info_extends.yml`
- `selected_completion_info_conflict.yml`
- `comment_no_completion.yml`
- `string_no_completion.yml`
- `heredoc_no_completion.yml`
- `pod_no_completion.yml`
- `module_import_workspace.yml`
- `test_more_assertion.yml`
- `test2_assertion.yml`
- `constructor_shift_style.yml`
- `constructor_signature_style.yml`
- `self_method_current_package.yml`
- `dbi_receiver_hint.yml`

Fixtures should cover both expected text and suppression. Suppression fixtures
are as important as positive fixtures.

## Local Measurement

Quality counters may be added for local/dev receipts. They must not upload
telemetry.

Useful counters:

- shown;
- accepted;
- partially accepted;
- dismissed;
- superseded by typing;
- parse-safe rejected;
- rejected by selected completion;
- rejected by hard zone.

Counters should be keyed by candidate source and rejection reason, not user
code.

## PR Ladder

Build the rails before expanding intelligence.

This ladder is historical and directional. Several phases are already complete
or partially complete in the current tree; do not reopen them unless a failing
test, receipt, or real editor report shows drift. Future work should extend the
implemented rails instead of replacing them.

| Phase | PR shape | Scope |
| --- | --- | --- |
| 0 | `docs(inline): define semantic inline-completion roadmap` | This document and related docs pointers. |
| 1 | `test(inline): lock LSP inline-completion protocol contracts` | Static, dynamic, disabled, automatic/invoked, shutdown, empty/null behavior. |
| 2 | `fix(inline): return editor-safe replacement ranges` | Prefix replacement, UTF-16 positions, single-line range safety. |
| 3 | `fix(inline): suppress ghost text that conflicts with selected completions` | `selectedCompletionInfo` extension/range contract. |
| 4 | `fix(inline): suppress code ghost text in comments strings heredocs and POD` | Hard reject zones and negative fixtures. |
| 5 | `feat(inline): build semantic context for inline candidates` | Internal `SemanticInlineContext` scaffold. |
| 6 | `feat(inline): detect file role and local Perl style` | Module/script/test role and style facts. |
| 7 | `feat(inline): use visible lexicals in deterministic suggestions` | Visible lexical facts and negative out-of-scope tests. |
| 8 | `refactor(inline): split deterministic candidates into sources` | Candidate source trait with behavior-preserving split. |
| 9 | `feat(inline): suggest workspace modules for use completions` | Effective include context and project module suggestions. |
| 10 | `feat(inline): suggest Perl test assertions from file context` | Test::More and Test2 assertion source. |
| 11 | `feat(inline): suggest current package methods for self receiver` | `$self->` current package method suggestions. |
| 12 | `feat(inline): rank candidates with semantic confidence` | Scored ranking and fixture snapshots. |
| 13 | `feat(inline): reject candidates that worsen local parse state` | Parse-safety filter. |
| 14 | `test(inline): add fixture-based UX tests for ghost text` | YAML or equivalent editor UX corpus. |
| 15 | `feat(inline): suggest constructor bodies matching local style` | Constructor idiom source. |
| 16 | `feat(inline): infer basic DBI receiver methods` | Narrow DBI receiver hints. |
| 17 | `feat(inline): suggest guard and loop continuations from visible context` | Control-flow continuations using visible facts. |
| 18 | `feat(inline): add local dev counters for inline completion quality` | Local-only counters. |
| 19 | `ci(inline): emit inline completion quality receipts` | Fixture totals, source counters, latency, parse regressions. |
| 20 | `feat(inline): add gated next-edit suggestion scaffold` | Feature-gated scaffold only. |
| 21 | `feat(inline): add guarded AI candidate source boundary` | Optional AI boundary after deterministic path is trusted. |

Current completed or substantially implemented phases:

- protocol contracts, release-built stdio smokes, and static/dynamic/disabled
  paths;
- replacement ranges and UTF-16 wire-position checks;
- `selectedCompletionInfo` alignment;
- hard reject zones;
- semantic context, file role, style context, visible lexical facts, and
  source-split deterministic candidates;
- workspace module imports, test assertions, current-package `$self->` methods,
  constructor style, DBI receiver hints, and visible-context loop/guard
  continuations;
- ranking, parse-safety filtering, and local quality receipts.

Still future or deliberately gated:

- larger real-project UX receipts for inline quality beyond deterministic unit
  fixtures;
- next-edit suggestions;
- optional AI candidate boundaries.

## High-Value Perl Wins

The first semantic wins should be Perl-specific and narrow:

- after `use My::`, suggest reachable workspace modules from effective include
  context;
- after `$self->`, suggest methods from the current package first;
- in `.t` files, suggest assertion forms that match Test::More or Test2 style;
- inside `sub new`, follow the constructor idiom already used in the file;
- inside unsupported zones, stay silent.

These are more valuable than broad generic completions because they prove the
server understands the user's Perl project.

## Validation

For docs-only roadmap changes:

```bash
cargo xtask check-devex-docs
cargo xtask doc-claims
cargo xtask check-support-claims
git diff --check
```

For behavior changes, use the release gate plus the narrow provider/core tests
listed in the relevant PR.

## Claim Boundary

This document is a roadmap and lane contract. It does not change runtime
behavior, advertised capabilities, release state, support tier, or editor setup.

It may claim:

- the intended semantic inline-completion direction;
- the order in which the lane should build protocol safety, context, sources,
  ranking, parse safety, local receipts, next-edit scaffolding, and optional AI.
- implemented semantic candidate behavior when backed by named tests, smokes, or
  quality receipts.

It may not claim:

- that semantic inline completion is complete across all Perl styles, project
  layouts, or editor integrations;
- that AI is enabled or planned as default behavior;
- that inline completion is generally perfect or production-complete;
- complete LSP 3.18 conformance;
- release readiness.
