# PLSP-ADR-0004: lsp-stack extraction boundary

Status: accepted
Date: 2026-05-26
Owner: perl-lsp maintainers
Linked proposal: n/a
Linked specs:
- [PLSP-SPEC-0028](../specs/PLSP-SPEC-0028-lsp-stack-extraction.md)
Linked plan: [lsp-stack extraction implementation plan](../../plans/lsp-stack-extraction/implementation-plan.md)

## Context

`perl-lsp` now has the current-app protocol and runtime hardening required
before any reusable LSP stack extraction:

- inline completion selects one registration mode for static and dynamic
  clients
- LSP 3.18 inline-completion parameter handling has runtime proof
- runtime watcher registration honors lean and e2e tuning
- semantic-token capabilities advertise full-only behavior until delta support
  has real result-id state
- raw RPC and lean editor receipt paths exist for the current app
- editor docs and release notes describe the current integration behavior

Those changes make the current product surface coherent enough to define a
future extraction boundary. They do not by themselves create a reusable stack,
prove release readiness, or authorize code movement.

## Decision

Future `lsp-stack` work must start from a written boundary, not from moving
files.

The reusable stack seam is the language-neutral LSP infrastructure that can be
shared without knowing Perl. The seam may eventually include:

- JSON-RPC message and request-id discipline
- LSP message framing and transport helpers
- server-originated request helpers
- capability-shape and dynamic-registration primitives
- lifecycle and runtime-tuning primitives that are not tied to Perl providers
- cancellation and scheduling primitives that do not encode Perl semantics
- test harness utilities for protocol contracts

Perl-specific behavior stays in the current app unless a later spec proves a
safe boundary. That includes parser, lexer, semantic analysis, workspace index,
provider behavior, feature catalog, inline-completion behavior, editor
receipts, DAP, packaging, and release automation.

## Preconditions

Extraction may begin only after current protocol, runtime, and editor-doc
hardening is complete and proven in the app that ships today.

The first extraction implementation PR must confirm that these current-app
contracts remain true before moving code:

- static inline-completion clients receive only the static provider
- dynamic inline-completion clients receive only dynamic registration
- disabled inline completion disables both static and dynamic registration
- e2e and lean runtime modes do not register file watchers
- file-watcher tuning does not suppress inline-completion registration
- semantic tokens do not advertise delta without result-id-backed delta support
- raw RPC and lean editor receipts continue to pass for the current app
- docs still describe standard inline completion and the custom
  `perlInlineCompletionStream` extension accurately

## Dependency Boundary

A future `lsp-stack` crate must not depend on Perl crates, Perl providers, Perl
runtime tooling, or Perl release surfaces.

Explicitly forbidden dependencies for a future `lsp-stack` include:

- `perl-lsp-rs`
- `perl-lsp-rs-core`
- `perl-parser`
- `perl-lexer`
- `perl-semantic-analyzer`
- `perl-workspace-index`
- `perl-module-*`
- `perl-lsp-*` feature or provider crates
- `perl-dap-*`
- `perl-subprocess-runtime`
- `perl-lsp-perltidy`
- any crate whose public contract requires Perl source, Perl workspace state,
  Perl provider facts, Perl debugger state, or Perl release artifacts

Allowed dependencies must be language-neutral infrastructure dependencies. Any
new dependency for `lsp-stack` requires a separate dependency-boundary review.

## Non-goals

This ADR does not authorize:

- creating `crates/lsp-stack`
- moving protocol, transport, router, lifecycle, scheduler, or provider files
- rewriting the router
- introducing generic handler traits
- extracting inline-completion types or provider logic
- extracting capability descriptors from the Perl feature catalog
- extracting DAP
- implementing semantic-token delta
- implementing true incremental parsing
- changing release, publish, signing, marketplace, or package metadata
- claiming release readiness

## Consequences

Positive consequences:

- agents have a durable map before extraction starts
- current-app behavior remains the regression baseline
- future stack code must stay language-neutral
- Perl-specific product behavior remains reviewable in the app until a proven
  seam exists

Tradeoffs:

- extraction starts later than a direct file move
- some infrastructure remains in `perl-lsp-rs-core` until the boundary is
  proven by tests
- generic handler abstractions remain out of scope until a separate design
  proves they reduce real complexity

## Alternatives Considered

### Create `crates/lsp-stack` immediately

Rejected. The current need is a boundary and proof map. Creating the crate
before the current-app contracts are documented would invite broad file
movement before the regression baseline is clear.

### Extract all protocol and runtime modules at once

Rejected. The protocol/runtime surface mixes language-neutral infrastructure
with Perl-specific feature wiring and current editor receipts. A bulk move
would obscure which behavior is reusable and which behavior is product-specific.

### Extract through generic handler traits first

Rejected for this tranche. Handler traits may or may not be useful after the
language-neutral seam is identified. They are not a prerequisite for documenting
the extraction boundary.

## Follow-up Obligations

- Keep [PLSP-SPEC-0028](../specs/PLSP-SPEC-0028-lsp-stack-extraction.md) as the
  acceptance contract for extraction PRs.
- Keep the implementation plan under
  [plans/lsp-stack-extraction](../../plans/lsp-stack-extraction/implementation-plan.md)
  as the PR sequence map.
- Require future extraction PRs to state whether they move code, change runtime
  behavior, change capability behavior, add dependencies, or alter release
  surfaces.
- Keep release readiness claims out of extraction PRs unless a separate release
  lane proves them.

## Status Links

- [LSP interactive latency rollout](../development/LSP_INTERACTIVE_LATENCY_ROLLOUT.md)
- [Editor setup](../how-to/EDITOR_SETUP.md)
- [IntelliJ IDEA setup](../EDITORS/INTELLIJ_IDEA_SETUP.md)
- [LSP capability contract ADR](0021-lsp-capability-contract.md)
- [Custom LSP runtime ADR](0034-custom-lsp-runtime.md)

## Why ADR-worthy

This is an architecture boundary decision. It defines when extraction may start,
what future reusable stack code may know, what must remain in `perl-lsp`, and
which tempting implementation shortcuts are intentionally out of scope.
