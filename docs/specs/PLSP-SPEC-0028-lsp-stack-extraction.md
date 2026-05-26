# PLSP-SPEC-0028: lsp-stack extraction boundary

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: n/a
Linked ADRs:
- [PLSP-ADR-0004](../adr/PLSP-ADR-0004-lsp-stack-extraction.md)
Linked plan: [lsp-stack extraction implementation plan](../../plans/lsp-stack-extraction/implementation-plan.md)
Status impact: protocol/runtime hardening, editor integration docs, future
crate-boundary reviews

## Current Implementation Status

This is a docs-only boundary spec. No `lsp-stack` crate exists because this spec
does not authorize implementation. The current app remains the source of truth
for LSP behavior until later extraction PRs prove equivalence.

The hardening prerequisite for extraction is the current-app protocol,
runtime, and editor-doc tranche:

- inline-completion registration mode is coherent for static, dynamic, and
  disabled clients
- LSP 3.18 inline-completion request shape has runtime coverage
- runtime watcher registration honors lean and e2e file-watcher tuning
- semantic-token capabilities no longer advertise delta without result-id state
- raw RPC and lean editor receipts exist for current-app behavior
- editor docs describe LSP4IJ, Neovim lean/e2e mode, standard inline
  completion, and the custom `perlInlineCompletionStream` extension

This spec does not claim release readiness.

## Contract

`lsp-stack` extraction is allowed only as a staged migration from proven
current-app behavior to language-neutral infrastructure.

The reusable seam may include only infrastructure that does not need Perl
source, Perl facts, Perl runtime state, Perl debugging state, or Perl release
state:

- JSON-RPC request and response envelopes
- strict request-id parsing and server-originated request-id allocation
- LSP framing and transport helpers
- server-originated request helper APIs
- capability JSON shape helpers that do not encode Perl features
- dynamic-registration helper APIs that do not encode Perl provider behavior
- runtime tuning and scheduling primitives that are language-neutral
- cancellation and stale-read primitives that do not inspect Perl documents
- protocol contract test utilities

The current app must retain ownership of:

- Perl parser, lexer, and semantic analyzer behavior
- workspace indexing and module resolution
- provider implementations and provider receipts
- `features.toml` and Perl capability catalog policy
- inline-completion provider behavior and stream payloads
- DAP and Perl debugger integration
- editor-specific integration docs
- release, publish, signing, marketplace, package, and installer surfaces

## Dependency Boundary

Future `lsp-stack` code must be dependency-inverted away from Perl.

It must not depend on:

- any crate named `perl-*`
- `perl-lsp-rs`
- `perl-lsp-rs-core`
- `perllsp`
- DAP crates from this workspace
- parser, lexer, semantic, workspace-index, module-resolution, perltidy, or
  subprocess-runtime crates from this workspace
- generated feature catalogs that encode Perl provider policy
- release or installer metadata

It may depend only on language-neutral protocol, serialization, error, test, or
runtime infrastructure after dependency review. Adding or widening dependencies
is a separate acceptance item and cannot be hidden inside a file-move PR.

## Valid PR Shapes

Valid PRs under this spec include:

- docs-only boundary, ADR, spec, or implementation-plan PRs
- static analysis PRs that report candidate seams without moving code
- test-only PRs that preserve current-app behavior before extraction
- mechanical no-behavior-change file moves after this spec's proof commands are
  green
- dependency-boundary PRs that remove Perl dependencies from candidate
  infrastructure before moving it
- post-move parity PRs that prove the same protocol/runtime behavior from the
  new boundary

Every extraction PR must state whether it changes:

- code location
- runtime behavior
- JSON-RPC behavior
- capability shape
- dynamic registration behavior
- editor integration behavior
- dependencies
- release or packaging surfaces

## Invalid PR Shapes

Invalid PRs include:

- creating `crates/lsp-stack` in the boundary-docs PR
- moving code before the current protocol/runtime/editor-doc hardening baseline
  is green
- bundling extraction with inline-completion behavior changes
- bundling extraction with DAP changes
- bundling extraction with release, publish, signing, marketplace, or installer
  changes
- adding generic handler traits as part of the boundary-docs PR
- rewriting routing as part of extraction setup
- weakening or deleting current inline-completion, watcher, semantic-token, raw
  RPC, or lean editor receipt tests
- introducing Perl dependencies into a future `lsp-stack`
- claiming release readiness from extraction docs or file movement

## Future PR Sequence

The intended sequence is:

1. Boundary docs: add this ADR, spec, and implementation plan. No code
   movement.
2. Static seam audit: classify protocol/runtime files as language-neutral,
   Perl-specific, mixed, or not extractable.
3. Dependency audit: prove candidate language-neutral files do not require Perl
   crates or feature-catalog policy.
4. Test baseline: keep current-app protocol/runtime/editor receipt commands
   green on `main`.
5. Crate scaffold: create `crates/lsp-stack` only after the audit PRs land.
6. First mechanical move: move one language-neutral, low-risk protocol module
   with no behavior change.
7. Transport/framing move: move framing helpers only after protocol parity is
   proven.
8. Runtime primitive move: move cancellation, request-id, or scheduling
   primitives only when they no longer encode Perl behavior.
9. Current-app integration: keep `perl-lsp` as the product crate and wire it to
   the extracted infrastructure with parity tests.
10. Post-extraction cleanup: remove duplicate wrappers only after behavior
    parity and dependency boundaries are proven.

Any step may be split smaller. No step may skip the proof commands for the
behavior it touches.

## Acceptance

An extraction-boundary PR satisfies this spec when it:

- adds only boundary docs, spec, and implementation-plan artifacts
- states that no code moved
- states that no runtime behavior changed
- states that no extraction implementation exists yet
- defines the reusable stack seam
- defines the Perl-specific non-extractable surface
- bans Perl dependencies in future `lsp-stack`
- defines future PR order
- defines proof commands
- defines rollback rules
- avoids release-readiness claims

A future implementation PR satisfies this spec only when it:

- starts after the current-app hardening baseline is green
- moves or changes one bounded surface
- proves current-app behavior still passes
- proves the candidate code has no Perl dependencies
- states rollback steps
- leaves unrelated providers, DAP, release, and editor docs alone

## Proof Commands

Docs-only boundary PRs must run:

```bash
git diff --check
```

When stable in the checkout, also run:

```bash
cargo xtask docs-check
```

When touched docs could affect support claims, run:

```bash
cargo xtask check-support-claims
```

Future implementation PRs must also run the current-app protocol and runtime
proof relevant to the moved surface, including targeted tests for:

```bash
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_inline_completion_registration_tests --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_registration_tests --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_cap_snap --profile agent --locked
./scripts/cargo-safe check -p perl-lsp-rs --all-targets --profile agent --locked
./scripts/cargo-safe check -p perl-lsp-rs-core --all-targets --profile agent --locked
```

Raw RPC and lean editor receipt commands are required for extraction PRs that
touch runtime scheduling, watcher registration, diagnostics timing,
initialization, request dispatch, or cancellation.

Report unrelated pre-existing warnings separately from extraction failures.

## Rollback Rules

Boundary docs rollback:

- revert the docs-only PR
- keep current app behavior unchanged
- do not remove current protocol/runtime tests

Implementation rollback:

- revert the smallest extraction PR that introduced the regression
- restore current-app imports and module paths
- keep the failing parity test or add a smaller regression test before retrying
- do not repair extraction by weakening current-app behavior
- do not remove proof commands from PR bodies to make rollback look green

Dependency rollback:

- remove the dependency from future `lsp-stack`
- move any Perl-specific code back to the current app
- document why the candidate was mixed rather than language-neutral

## Non-goals

This spec does not authorize:

- creating `crates/lsp-stack`
- moving code
- rewriting routing
- introducing generic handler traits
- extracting inline-completion implementation
- extracting DAP
- changing release or publish automation
- changing package metadata
- claiming release readiness

## Claim Boundaries

This spec may claim only that the extraction boundary is documented. It must
not claim:

- an extracted reusable stack exists
- extraction has started
- runtime behavior changed
- editor integrations are more ready than current receipts prove
- release, publish, package, signing, marketplace, or installer readiness
