# PLSP-SPEC-0029: LSP 3.18 conformance boundary

Status: accepted
Owner: perl-lsp maintainers
Linked proposal: n/a
Linked ADRs:
- [PLSP-ADR-0004](../adr/PLSP-ADR-0004-lsp-stack-extraction.md)
Linked plan: n/a
Status impact: LSP capability claims, editor integration receipts, protocol
contract tests, future extraction parity reviews

## Current Implementation Status

This spec records the current claim boundary for selected LSP 3.18 surfaces in
`perl-lsp`. It is a support-boundary document, not a release approval and not a
claim of complete LSP 3.18 implementation.

The upstream LSP 3.18 specification is still marked upcoming and under
development. The project treats 3.18 support as capability-negotiated claim
honesty: every advertised 3.18 behavior must be shaped correctly, routed
correctly, tested over JSON-RPC, and documented; every unsupported 3.18 behavior
must stay absent from capabilities or return the standard unsupported or invalid
params error.

Spec source: [Language Server Protocol Specification - 3.18](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/).

Current lock points:

- inline completion uses standard `textDocument/inlineCompletion`
- static clients receive top-level `inlineCompletionProvider`
- dynamic-capable clients receive `client/registerCapability` for
  `textDocument/inlineCompletion` after `initialized`
- `experimental.inlineCompletionProvider` is not used
- `experimental.perlInlineCompletionStream` remains a custom extension
- multi-range formatting is advertised through
  `documentRangeFormattingProvider.rangesSupport`
- `documentRangesFormattingProvider` is not advertised
- semantic tokens advertise full/range support without delta
- signature-help active parameter schema validation accepts unsigned integer or
  `null`
- `workspace/textDocumentContent` is wired for the `perldoc` scheme
- lean/e2e runtime mode suppresses file watcher registration without suppressing
  unrelated inline-completion dynamic registration

## Contract

`perl-lsp` may claim selected LSP 3.18 support only for surfaces with:

- a capability JSON path
- a routed method or dynamic registration path
- disabled-feature behavior when the feature is feature-gated
- a wire-level JSON-RPC test
- a capability snapshot or shape assertion
- negative unsupported behavior for adjacent unimplemented 3.18 surfaces
- editor receipt coverage when the feature affects real editor startup or use

The project must not use "LSP 3.18 compliant" as a blanket claim. The supported
claim is narrower:

```text
perl-lsp supports selected LSP 3.18 surfaces with capability-honest contracts.
```

## Supported And Locked Surfaces

| Surface | Capability or method | Current contract | Proof |
| --- | --- | --- | --- |
| Inline completion | `inlineCompletionProvider`, `textDocument/inlineCompletion`, `client/registerCapability` | Static and dynamic modes are mutually exclusive; disabled inline completion removes provider, stream flag, dynamic registration, and runtime handling. | `lsp_inline_completion_registration_tests`, `lsp_ai_inline_completion_tests`, `lsp_streaming_completion_tests`, `lsp_cap_snap` |
| Inline completion selected context | `selectedCompletionInfo` | Returned items must use the same range and extend selected text, or return empty. | `lsp_inline_completion_registration_tests`, `lsp_inline_completion_tests` |
| Multi-range formatting | `documentRangeFormattingProvider.rangesSupport`, `textDocument/rangesFormatting` | Multi-range formatting uses the spec capability shape and routed method; the non-spec plural capability is absent. | `lsp_caps_contract_shapes`, `lsp_disabled_features_tests`, `lsp_formatting_e2e`, `lsp_capabilities_snapshot`, `lsp_cap_snap` |
| Dynamic text document content | `workspace.textDocumentContent.schemes`, `workspace/textDocumentContent` | `perldoc` scheme is advertised; invalid params and malformed URIs return `InvalidParams`; unsupported schemes return deterministic unavailable errors. | `lsp_text_document_content_tests`, `lsp_cap_snap` |
| Text document content refresh | `workspace/textDocumentContent/refresh` | Server-originated request IDs are bounded and emitted through the standard server request path. | `lsp_text_document_content_tests` |
| Folding range refresh | `workspace.foldingRange.refreshSupport`, `workspace/foldingRange/refresh` | Server sends refresh requests only for clients that advertise `workspace.foldingRange.refreshSupport`; request IDs are bounded and emitted through the standard server request path. | `lsp_refresh_methods_tests`, `lsp_318_negative_claims`, `check-lsp-318-claims` |
| Semantic tokens | `semanticTokensProvider.full`, `semanticTokensProvider.range` | Full and range are advertised; delta is not advertised without result-id state. | `lsp_caps_contract_shapes`, `lsp_semantic_legend_contract_tests`, `lsp_cap_snap` |
| Signature-help nullable active parameter | `textDocument/signatureHelp` response | `SignatureHelp.activeParameter` and `SignatureInformation.activeParameter` schema validation accepts unsigned integer or `null`; current runtime receipts preserve numeric active-parameter tracking when known. | `lsp_schema_validation`, `lsp_signature_help_tests`, `check-lsp-318-claims` |
| Lean/e2e watcher behavior | `workspace/didChangeWatchedFiles` dynamic registration | Runtime tuning can suppress file watchers without suppressing inline-completion dynamic registration. | `lsp_registration_tests`, lean UX receipts |

## Explicitly Unclaimed Surfaces

These surfaces are not part of the current claim unless a later PR adds behavior,
capability parsing, wire tests, docs, and negative gates:

- complete LSP 3.18 implementation
- `textDocument/semanticTokens/full/delta`
- semantic-token delta `resultId` state
- `WorkspaceEdit` `SnippetTextEdit`
- `WorkspaceEdit` metadata
- `ApplyWorkspaceEditParams.metadata`
- `CompletionList.applyKind`
- `CompletionList.itemDefaults.data`
- `CodeAction.documentation`
- `CodeAction.tags`
- `CodeActionTag.LLMGenerated`
- `MessageType.Debug`
- `Command.tooltip`
- `RelativePattern` document selectors and watcher glob patterns
- ungated `workspace/foldingRange/refresh` without
  `workspace.foldingRange.refreshSupport`
- trusted markdown command execution or theme-icon markdown behavior
- notebook-specific 3.18 additions beyond existing notebook sync claims

Unsupported or unclaimed surfaces must be absent from capabilities and from
representative responses unless the client capability and server behavior are
both implemented and tested.

## Negative Claim Gates

The `lsp_318_negative_claims` test suite is the current guardrail for optional
3.18 surfaces. It must fail if the server accidentally:

- advertises semantic-token delta
- accepts `textDocument/semanticTokens/full/delta` as implemented
- reintroduces `experimental.inlineCompletionProvider`
- reintroduces `documentRangesFormattingProvider`
- emits `CompletionList.applyKind` without explicit support
- emits `CompletionList.itemDefaults.data` without explicit support
- emits `CodeAction.documentation` or `CodeAction.tags`
- emits workspace-edit metadata or snippet edits in representative edit
  responses
- emits diagnostic `message` as `MarkupContent` without markup support
- registers file watchers with relative-pattern objects instead of string globs
- sends `workspace/foldingRange/refresh` without client refresh support
- emits `MessageType.Debug`

The suite is intentionally absence-first. It does not implement the optional
features and must not be treated as proof that those features work.

## Valid PR Shapes

Valid PRs under this spec include:

- adding or tightening wire tests for an already advertised 3.18 surface
- adding negative gates for unimplemented 3.18 structures
- correcting capability JSON shape to match the upstream specification
- documenting the current support boundary
- adding a claim-checking `xtask` that enforces this spec
- adding one optional 3.18 feature only when the PR includes capability parsing,
  capability advertisement, request or response behavior, disabled-feature
  behavior when applicable, wire tests, docs, and negative tests for disabled or
  unsupported clients

Every PR must state whether it changes capability shape, dynamic registration,
runtime behavior, response shape, docs, editor receipts, extraction boundaries,
or release surfaces.

## Invalid PR Shapes

Invalid PRs include:

- claiming full LSP 3.18 implementation from selected-surface receipts
- advertising a 3.18 capability before the routed behavior and wire tests exist
- emitting client-gated response fields before the client capability is parsed
- implementing semantic-token delta without result-id state
- bundling 3.18 optional feature work with `lsp-stack` extraction
- creating `crates/lsp-stack`
- moving protocol or routing code as part of this spec
- touching DAP
- touching release, publish, signing, package, marketplace, or installer
  behavior
- weakening inline-completion, range-formatting, text-document-content,
  semantic-token, watcher, or editor receipt tests
- claiming release readiness from this spec

## Acceptance

This spec is satisfied when:

- supported 3.18 surfaces have capability-shape and wire-contract tests
- unimplemented optional surfaces have negative gates
- docs name selected support rather than blanket conformance
- extraction remains downstream of current-app behavior parity
- every later 3.18 PR updates this spec or its proof references when it changes
  the support boundary

## Proof Commands

For this docs-only boundary:

```bash
git diff --check
cargo xtask check-support-claims
cargo xtask check-lsp-318-claims
cargo xtask generate-lsp-318-matrix --check
cargo xtask docs-check
```

If a command is unstable in the checkout, the PR must report that separately.

For behavior or test PRs touching this boundary, run the relevant subset:

```bash
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_inline_completion_registration_tests --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_ai_inline_completion_tests --features expose_lsp_test_api --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_caps_contract_shapes --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_capabilities_snapshot --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_cap_snap --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_registration_tests --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_text_document_content_tests --profile agent --locked
./scripts/cargo-safe test -p perl-lsp-rs --test lsp_318_negative_claims --profile agent --locked
cargo xtask check-lsp-318-claims
cargo xtask generate-lsp-318-matrix --check
./scripts/cargo-safe check -p perl-lsp-rs --all-targets --profile agent --locked
git diff --check
./scripts/storage-doctor
```

Editor receipt refresh PRs must additionally run the raw RPC, lean Neovim, and
inline-completion binary smoke commands relevant to the touched editor surface.

## Non-goals

- No full LSP 3.18 implementation claim.
- No broad optional 3.18 feature implementation.
- No semantic-token delta implementation.
- No `lsp-stack` extraction.
- No routing rewrite.
- No generic handler trait introduction.
- No DAP changes.
- No release, publish, signing, package, marketplace, or installer changes.
- No release-readiness claim.

## Claim Boundaries

This spec may claim that `perl-lsp` has a documented LSP 3.18 selected-surface
support boundary and negative gates for unimplemented optional surfaces.

It may not claim:

- complete LSP 3.18 conformance
- release readiness
- extraction readiness beyond the separate extraction boundary spec
- editor support beyond current receipts
- semantic-token delta support
- workspace-edit snippet or metadata support
- optional 3.18 response-shape support without client capability handling
