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

The current upstream 3.18 spec does not define VS Code-style markdown theme
icons or trusted markdown command links as LSP capabilities. `supportThemeIcons`
and trusted markdown `enabledCommands` therefore remain defensive absence
guards, not 3.18 implementation targets.

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
- semantic-token legends include LSP 3.18 `SemanticTokenTypes.label` only
  because the provider emits matching label token indexes
- signature-help active parameter schema validation accepts unsigned integer or
  `null`
- CodeLens command lazy-resolution is gated by
  `textDocument.codeLens.resolveSupport.properties`
- CodeLens commands include plain LSP 3.18 `Command.tooltip` text
- explicit window debug messages serialize LSP 3.18 `MessageType.Debug` as
  type `5`
- pull diagnostics can emit `Diagnostic.message` as `MarkupContent` only when
  clients advertise `textDocument.diagnostic.markupMessageSupport`
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
| Dynamic text document content | `workspace.textDocumentContent.schemes`, `workspace/textDocumentContent` | `perldoc` scheme is advertised; local workspace POD returns extracted sections plus sorted related `perldoc://` links for simple module references and supported core pragmas; invalid params and malformed URIs return `InvalidParams`; unsupported schemes return deterministic unavailable errors. | `lsp_text_document_content_tests`, `lsp_cap_snap` |
| Text document content refresh | `workspace/textDocumentContent/refresh` | Server-originated request IDs are bounded and emitted through the standard server request path. | `lsp_text_document_content_tests` |
| Folding range refresh | `workspace.foldingRange.refreshSupport`, `workspace/foldingRange/refresh` | Server sends refresh requests only for clients that advertise `workspace.foldingRange.refreshSupport`; request IDs are bounded and emitted through the standard server request path. | `lsp_refresh_methods_tests`, `lsp_318_negative_claims`, `check-lsp-318-claims` |
| Semantic tokens | `semanticTokensProvider.full`, `semanticTokensProvider.range` | Full and range are advertised; delta is not advertised without result-id state. | `lsp_caps_contract_shapes`, `lsp_semantic_legend_contract_tests`, `lsp_cap_snap` |
| Semantic token `label` type | `semanticTokensProvider.legend.tokenTypes` | `SemanticTokenTypes.label` is advertised and emitted for deterministic Perl label declarations and loop-control label references; emitted token indexes and modifier bits stay within the advertised legend bounds. | `lsp_semantic_legend_contract_tests`, `check-lsp-318-claims` |
| Signature-help nullable active parameter | `textDocument/signatureHelp` response | `SignatureHelp.activeParameter` and `SignatureInformation.activeParameter` schema validation accepts unsigned integer or `null`; current runtime receipts preserve numeric active-parameter tracking when known. | `lsp_schema_validation`, `lsp_signature_help_tests`, `check-lsp-318-claims` |
| SnippetTextEdit workspace edits | `workspace.workspaceEdit.documentChanges`, `workspace.workspaceEdit.snippetEditSupport` | Pragma quick-fix code actions emit `SnippetTextEdit` in `WorkspaceEdit.documentChanges` only when both capabilities are present; unsupported clients and aggregate fix-all actions keep plain `TextEdit` fallback. | `lsp_318_negative_claims`, `features.toml`, `check-lsp-318-claims` |
| CodeLens resolve support properties | `textDocument.codeLens.resolveSupport.properties`, `codeLensProvider.resolveProvider`, `codeLens/resolve` | Clients receive unresolved command/reference lenses only when `command` appears in resolve-support properties; clients without that property receive eager command lenses while `codeLens/resolve` remains routed. | `lsp_codelens_tests`, `lsp_code_lens_tests`, `lsp_bdd_workflows`, `check-lsp-318-claims` |
| CodeLens command tooltips | `Command.tooltip` on CodeLens command objects | CodeLens commands returned by `textDocument/codeLens` and `codeLens/resolve` carry deterministic plain-text tooltips; non-CodeLens command tooltips remain unclaimed. | `lsp_codelens_tests`, `lsp_318_negative_claims`, `check-lsp-318-claims` |
| Completion list default data | `textDocument.completion.completionList.itemDefaults`, `textDocument/completion` | Clients that include `data` in supported completion-list defaults receive shared `CompletionList.itemDefaults.data`; unsupported clients retain the current response shape. | `lsp_completion_tests`, `lsp_318_negative_claims`, `check-lsp-318-claims` |
| Completion list apply kind | `textDocument.completion.completionList.applyKindSupport`, `textDocument/completion` | Clients that support apply kind and `itemDefaults.data` receive `CompletionList.applyKind.data = 2` (`ApplyKind.Merge`); unsupported clients, or clients without supported defaults, receive no `applyKind`. | `lsp_completion_tests`, `lsp_318_negative_claims`, `check-lsp-318-claims` |
| CodeAction documentation | `textDocument.codeAction.documentationSupport`, `codeActionProvider.documentation` | Clients that support code-action documentation receive `CodeActionOptions.documentation` for `quickfix`, `refactor`, and `source.fixAll`; unsupported clients receive no documentation advertisement and individual code-action responses remain unchanged. | `lsp_318_negative_claims`, `check-lsp-318-claims` |
| CodeAction tag trust boundary | `textDocument.codeAction.tagSupport.valueSet`, `CodeAction.tags` | The server parses support for `CodeActionTag.LLMGenerated`, strips unsupported or malformed tag payloads from code-action and resolve responses, and verifies deterministic code actions remain untagged even for tag-capable clients. Actual generated-action tagging remains unclaimed until a generated-action source exists. | `lsp_318_negative_claims`, `check-lsp-318-claims` |
| Apply-edit metadata | `workspace.applyEdit`, `workspace.workspaceEdit.metadataSupport`, `workspace/applyEdit` | Server-originated refactoring apply-edit requests may include `ApplyWorkspaceEditParams.metadata.isRefactoring` only when both capabilities are present; ordinary `WorkspaceEdit` responses stay metadata-free. | `lsp_318_negative_claims`, `features.toml`, `check-lsp-318-claims` |
| Window debug messages | `MessageType.Debug`, `window/logMessage`, `window/showMessage`, `window/showMessageRequest` | Explicit debug message calls serialize type `5`; normal runtime paths continue using the existing non-debug message levels unless a later PR intentionally wires debug policy. | `lsp_window_tests`, `lsp_318_negative_claims`, `check-lsp-318-claims` |
| Diagnostic markup messages | `textDocument.diagnostic.markupMessageSupport`, `textDocument/diagnostic`, `workspace/diagnostic` | Pull diagnostics may emit `Diagnostic.message` as `MarkupContent` only when support is true; unsupported clients and publish diagnostics remain string-only. | `lsp_diagnostic_enrichment_test`, `lsp_318_negative_claims`, `lsp_schema_validation`, `check-lsp-318-claims` |
| Lean/e2e watcher behavior | `workspace/didChangeWatchedFiles` dynamic registration | Runtime tuning can suppress file watchers without suppressing inline-completion dynamic registration. | `lsp_registration_tests`, lean UX receipts |
| RelativePattern watcher registrations | `workspace.didChangeWatchedFiles.relativePatternSupport`, `workspace/didChangeWatchedFiles` dynamic registration | Clients that support relative watcher glob patterns receive `baseUri`/`pattern` objects rooted at workspace folders; unsupported clients and invalid workspace roots keep string glob fallback. | `lsp_registration_tests`, `lsp_318_negative_claims`, `check-lsp-318-claims` |

## Matrix Closeout State

The generated LSP 3.18 matrix is closed for the current support boundary. Every
row is classified as one of:

- `implemented+tested+documented`
- `negative-gated+documented`
- `not-applicable+documented`

Rows must not use transitional statuses such as "needs capability parser" or
"planned needs negative gate" unless a later PR intentionally reopens the matrix
with a documented follow-up lane. `cargo xtask check-lsp-318-claims` enforces
that the checked-in matrix stays in this closed-state vocabulary.

## Explicitly Unclaimed Surfaces

These surfaces are not part of the current claim unless a later PR adds behavior,
capability parsing, wire tests, docs, and negative gates:

- complete LSP 3.18 implementation
- object-form `StringValue` inline completion insert text
- `textDocument/semanticTokens/full/delta`
- semantic-token delta `resultId` state
- non-spec `WorkspaceEdit.metadata` response fields
- generated-action `CodeAction.tags` emission
- `CodeActionTag.LLMGenerated` on deterministic actions
- `Command.tooltip` outside CodeLens command objects
- `RelativePattern` document selectors
- ungated `workspace/foldingRange/refresh` without
  `workspace.foldingRange.refreshSupport`
- VS Code-style markdown command links, theme-icon syntax, `supportThemeIcons`,
  or trusted markdown `enabledCommands` as LSP 3.18 capabilities
- notebook-specific 3.18 additions beyond existing notebook sync claims

Unsupported or unclaimed surfaces must be absent from capabilities and from
representative responses unless the client capability and server behavior are
both implemented and tested.

For the current 3.18 metadata boundary, upstream metadata is
`ApplyWorkspaceEditParams.metadata` on a server-originated
`workspace/applyEdit` request. It is not a field on ordinary `WorkspaceEdit`
responses returned from `textDocument/rename`, `textDocument/codeAction`, or
file-operation requests.

## Negative Claim Gates

The `lsp_318_negative_claims` test suite is the current guardrail for optional
3.18 surfaces. It must fail if the server accidentally:

- advertises semantic-token delta
- accepts `textDocument/semanticTokens/full/delta` as implemented
- reintroduces `experimental.inlineCompletionProvider`
- reintroduces `documentRangesFormattingProvider`
- emits object-form `StringValue` values for
  `InlineCompletionItem.insertText` without an intentional implementation
- emits `CompletionList.applyKind` without explicit support
- emits `CompletionList.itemDefaults.data` without explicit support
- advertises `CodeAction.documentation` without client support or emits
  `CodeAction.tags` without an explicit generated-action source and client
  `tagSupport`
- emits non-spec `WorkspaceEdit.metadata` fields in representative edit
  responses
- emits `ApplyWorkspaceEditParams.metadata` without a gated
  `workspace/applyEdit` server-request path
- emits `SnippetTextEdit` without explicit support
- emits diagnostic `message` as `MarkupContent` without markup support
- registers file watchers with relative-pattern objects without
  `workspace.didChangeWatchedFiles.relativePatternSupport`
- sends `workspace/foldingRange/refresh` without client refresh support
- emits `MessageType.Debug` from normal runtime paths that have not
  intentionally opted into debug-level messages
- emits `Command.tooltip` outside CodeLens command objects
- emits markdown `command:` links or `$()` theme-icon syntax while the project
  has no separate editor-specific capability contract for those affordances

The suite is intentionally absence-first for still-unclaimed surfaces. Positive
receipts for implemented optional features live beside the relevant wire tests
and are checked by `cargo xtask check-lsp-318-claims`.

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
support boundary and negative gates for unimplemented optional surfaces,
including:

- capability-gated `SnippetTextEdit` workspace edits in
  `WorkspaceEdit.documentChanges`, with plain `TextEdit` fallback for
  unsupported clients
- capability-gated `ApplyWorkspaceEditParams.metadata` on server-originated
  `workspace/applyEdit` requests

It may not claim:

- complete LSP 3.18 conformance
- release readiness
- extraction readiness beyond the separate extraction boundary spec
- editor support beyond current receipts
- semantic-token delta support
- object-form `StringValue` inline completion insert text
- non-spec `WorkspaceEdit.metadata` response fields
- ungated workspace-edit snippet or apply-edit metadata support
- optional 3.18 response-shape support without client capability handling
