# Inline Completion Contract Index

**Purpose.** One place an agent or PR author can ask:
*"What does perl-lsp guarantee about `textDocument/inlineCompletion` (LSP 3.18),
how does it map to the upstream spec, what tests prove it, and what must not be
broadened?"*

Every contract below names: the invariant that must hold, the owner module, the
consumers, the proof tests, known exceptions, and non-goals / future migrations.

This document is kept factual and citable. Claims without a primary artifact
(file path, test name, merged PR) are not made.

**Related**: [PARSER_CONTRACTS.md](PARSER_CONTRACTS.md) (parser behavioral
invariants), [DAP_CONTRACTS.md](DAP_CONTRACTS.md) (DAP wire-protocol codec),
[AI_COMPLETION.md](AI_COMPLETION.md) (AI backend configuration reference),
[LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md) §"Inline Completion
Capability Registration", [features.toml](../../features.toml) entries
`lsp.inline_completion` and `experimental.perlInlineCompletionStream`.

---

## 0. Upstream spec status and the `@proposed` convention

`textDocument/inlineCompletion` is a **proposed** feature in the upstream LSP
specification (LSP 3.18 preview), not a stabilized request. perl-lsp tracks this
distinction with two independent axes in `features.toml`:

- **`spec = "LSP 3.18"`** + the `(@proposed)` suffix in `description` — record the
  *upstream protocol* status. The method itself is proposed in the LSP spec.
- **`maturity = "ga"`** — records perl-lsp's *implementation* maturity. The
  perl-lsp implementation is complete, advertised, and regression-tested.

These two are **not** contradictory and must not be "reconciled" by deleting the
`(@proposed)` marker. The same convention is used for every other
upstream-proposed method perl-lsp implements at GA quality — e.g.
`textDocument/rangesFormatting` (`features.toml` `lsp.ranges_formatting`),
`workspace/foldingRange/refresh`, and `workspace/textDocumentContent`. Removing
`(@proposed)` would falsely imply the *upstream method* is stable.

**Invariant.** A `features.toml` entry whose `spec` names an LSP version where the
method is upstream-proposed keeps `(@proposed)` in its `description` regardless of
`maturity`. `maturity` describes perl-lsp; `(@proposed)` describes the LSP spec.

**Owner**: [features.toml](../../features.toml) lines for `lsp.inline_completion`
and the cross-referenced `lsp.ranges_formatting`, `lsp.text_document_content`.

---

## 1. Wire shapes — LSP 3.18 conformance

### Contract

The serialized request/response field **names and shapes** match the LSP 3.18
`inlineCompletion` types, with camelCase field names. One deliberate value-type
narrowing is documented below (`insertText` is plain string only).

**Response items** — `crates/perl-lsp-rs-core/src/providers/inline_completion/mod.rs`:

- `InlineCompletionItem` (struct at `mod.rs:295`, `#[serde(rename_all = "camelCase")]`):
  - `insert_text: String` → `insertText` (required)
  - `filter_text: Option<String>` → `filterText` (omitted when `None`)
  - `range: Option<lsp_types::Range>` → `range` (omitted when `None`)
  - `command: Option<lsp_types::Command>` → `command` (omitted when `None`)
- `InlineCompletionList` (struct at `mod.rs:312`): `items: Vec<InlineCompletionItem>`
  → `items`.

These four item **fields** are the complete LSP 3.18 `InlineCompletionItem`
surface. Optional fields are elided via `skip_serializing_if = "Option::is_none"`
so the emitted JSON is minimal and spec-shaped.

**Supported subset — `insertText`.** The LSP 3.18 spec types `insertText` as
`string | StringValue` (the `StringValue` object form, `{ kind: "snippet", value }`,
carries snippet-syntax insertions). perl-lsp implements the **plain `string`**
arm only — `insert_text` is a Rust `String`, so emitted items are always literal
text, never a snippet `StringValue`. This is a deliberate conformance subset, not
a typed `StringValue` struct. Emitting snippet insertions is a future extension
that must add the `StringValue` variant (and a client `insertTextFormat`/snippet
capability gate) before this contract can claim the full `string | StringValue`
surface.

**Request params** — parsed in
`crates/perl-lsp-rs/src/runtime/language/misc.rs::handle_inline_completion`
(`misc.rs:886`):

- `textDocument.uri` via `req_uri` (`misc.rs:897`)
- `position` via `req_position` (`misc.rs:898`) — UTF-16 code-unit columns per LSP
- `context.triggerKind` via `inline_completion_trigger_kind` (`misc.rs:157`)
- `context.selectedCompletionInfo` via `selected_inline_completion_info`
  (`misc.rs:168`)

### Proof tests

- `crates/perl-lsp-rs/tests/lsp_inline_completion_tests.rs`:
  `test_inline_completion_after_arrow`, `test_inline_completion_after_use`,
  `test_inline_completion_shebang`, `test_inline_completion_sub_body`,
  `test_inline_completion_no_suggestions`.
- UTF-16 / multibyte column correctness:
  `test_inline_completion_after_arrow_with_multibyte_prefix`,
  `test_inline_completion_utf16_position_correct`,
  `test_inline_completion_multiline_crlf_doc_line1`.

### Non-goals

The proposed `command` field is plumbed through the type but perl-lsp's
deterministic sources do not currently emit a `command`; it exists for AI-backend
and future use. Adding a command must not change the elision behavior.

---

## 2. Capability advertisement & dynamic registration

### Contract

`inlineCompletionProvider` is advertised as a top-level **empty object** in
server capabilities **only** when the build flag `inline_completion` is enabled.
When disabled, neither the static provider nor dynamic registration is offered.

- Static advertisement: `crates/perl-lsp-rs-core/src/protocol/capabilities.rs:89`
  injects `json["inlineCompletionProvider"] = {}` under `if build.inline_completion`.
  (Injected manually because `lsp-types 0.97` predates the 3.18 field —
  `capabilities.rs:85`, `:149`.)
- Runtime gate: `handle_inline_completion` returns `method_not_advertised()` when
  `self.advertised_features.lock().inline_completion` is false (`misc.rs:892`).
- Dynamic registration: when the client opts into dynamic registration the static
  provider is withdrawn — `crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs`
  (see §"Inline Completion Capability Registration" in
  [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)).

### Proof tests

- `capabilities.rs:232` `inline_completion_advertised_as_top_level_json_when_enabled`
- `capabilities.rs:243` `inline_completion_absent_when_disabled`
- `crates/perl-lsp-rs/tests/lsp_inline_completion_registration_tests.rs` (dynamic
  registration handshake).

### Invariant

The advertised shape is an **empty object**, not `true` and not a populated
options object. Advertisement and the runtime handler gate read the **same**
feature flag, so an advertised capability is always serviceable and a withdrawn
capability always rejects with `method_not_advertised`.

---

## 3. `triggerKind` auto-trigger policy

### Contract

`context.triggerKind` is mapped to an internal tri-state
(`InlineCompletionTriggerKind`, `misc.rs:60`):

| Wire value | Internal | Meaning |
|------------|----------|---------|
| `1`        | `Invoked` | explicit user invocation |
| `2`        | `Automatic` | typing-triggered |
| absent     | `LegacyNoContext` | pre-3.18 client, no `context` |
| any other  | — | `invalid_params` error (`misc.rs:163`) |

When `triggerKind == Automatic`, `apply_inline_completion_trigger_policy`
(`misc.rs:233`) retains only items judged safe for unsolicited ghost-text and
truncates to a single item. "Safe" (`is_safe_automatic_inline_item`, `misc.rs:245`)
means: non-empty trimmed text, ≤ 80 characters, ends with `;`, contains none of
`\r \n $ @ % { } [ ] ( )`, and no `...`. `Invoked` and `LegacyNoContext` requests
are returned unfiltered.

### Invariant

Automatic triggers never surface multi-line, sigil-bearing, or unterminated
suggestions as ghost text — these are reserved for explicit invocation. An unknown
positive `triggerKind` is a client error (`invalid_params`), not silently coerced.

### Proof / owner

`misc.rs:233`–`254`. Policy is applied **after** AI and deterministic paths
(`misc.rs:934`, `misc.rs:985`) so it governs both backends uniformly.

---

## 4. `selectedCompletionInfo` constraint

### Contract

When the client supplies `context.selectedCompletionInfo` (the widget item the
user has highlighted in the standard completion popup), inline items are
constrained by `constrain_inline_completions_to_selected_info` (`misc.rs:191`):

1. If the selected `range` spans multiple lines, **all** items are dropped
   (`misc.rs:201`) — the spec's selectedCompletionInfo range is single-line.
2. An item survives only if its `insertText` **starts with** the selected `text`
   (`misc.rs:215`).
3. Range agreement (`misc.rs:219`):
   - item has an explicit `range` equal to the selected range → keep;
   - item has a different explicit range → drop;
   - item has no range **and** the selected range is the empty range at the
     request position → adopt the selected range and keep;
   - item has no range and the selected range is non-empty/elsewhere → drop.

### Invariant

Inline ghost text never contradicts the popup selection the user is already
looking at: it must extend the selected text and agree on the replaced range.

### Proof / owner

`misc.rs:191`–`231`. Applied to both AI (`misc.rs:928`) and deterministic
(`misc.rs:974`) results before the trigger policy.

---

## 5. AI backend path & deterministic fallback

### Contract

`handle_inline_completion` tries a registered AI backend first **iff**
`config.ai_completion.enabled` (`misc.rs:917`), then falls back to the
deterministic provider. Fallback is governed by `ai_config.fallback`:

- AI returns non-empty → apply replacement ranges, selected-info constraint, and
  trigger policy (`misc.rs:921`–`934`). The handler returns the AI list **only if
  it is still non-empty after filtering, or `fallback` is false** — the guard is
  `if !list.items.is_empty() || !ai_config.fallback` (`misc.rs:935`). So when the
  selected-info constraint or the automatic-trigger policy filters every AI item
  out **and** `fallback` is true, the handler does **not** return the empty AI
  list; it falls through to the deterministic provider below. With `fallback`
  false it returns the (possibly empty) AI list.
- AI errors or returns empty → if `fallback` is false, return `{ "items": [] }`;
  otherwise fall through to the deterministic provider (`misc.rs:944`–`959`).
- Deterministic path always applies the same selected-info + trigger-policy
  pipeline (`misc.rs:962`–`991`).

The document text is snapshotted under the document lock and the lock released
before any slow backend work (`misc.rs:902`–`911`); a missing document yields
`{ "items": [] }` (`misc.rs:907`).

The deterministic provider (`InlineCompletionProvider`, `mod.rs`) draws from
ranked candidate sources (receiver, module, syntax, test, shebang, contextual
fallback) and is parse-safety filtered. Backend trait:
`InlineCompletionBackend` (`mod.rs:388`).

### Proof tests

- `crates/perl-lsp-rs/tests/lsp_ai_inline_completion_tests.rs` (backend path).
- Deterministic quality scenarios under `crates/perl-lsp-ux-tests/tests/`
  (`ux_scenario_5x`/`6x_*_inline_completion_quality.rs`).
- `test_inline_completion_uses_latest_changed_document_text`,
  `test_inline_completion_mid_code_uses_nearby_variable_context` in
  `lsp_inline_completion_tests.rs`.

### Configuration reference

See [AI_COMPLETION.md](AI_COMPLETION.md): `perl-lsp.aiCompletion.enabled`
(default `false`), timeout, and token controls.

---

## 6. Streaming extension — `textDocument/perlInlineCompletionStream`

### Contract

A **custom, non-standard** request `textDocument/perlInlineCompletionStream`
delivers incremental inline completions via `$/progress`. It is feature
`experimental.perlInlineCompletionStream` in `features.toml` (no LSP-spec
backing, so **no** `(@proposed)` marker — it is a perl-lsp extension, not an
upstream-proposed method).

- Handler: `handle_streaming_inline_completion`
  (`crates/perl-lsp-rs/src/runtime/language/streaming.rs:20`).
- Routed at `crates/perl-lsp-rs/src/runtime/dispatch/routing.rs:129`.
- Requires a `partialResultToken`; absent → one-shot fallback returning items
  directly (`streaming.rs`).
- Emits `$/progress` notifications with payload
  `{ token, value: { kind: "perlInlineCompletionStream", sessionId, sequence,
  isFinal, items } }` (`streaming.rs:99`, `:150`, `:197`). Each chunk carries
  **cumulative** text (not a delta).
- Session replacement is **scoped to the session key**. `StreamSessionManager::`
  `start_session` (`crates/perl-lsp-rs/src/runtime/stream_session.rs:83`) cancels
  the prior session only for the **same `SessionKey`** = (`uri`, `document_version`,
  `line`, `character`) (`stream_session.rs:15`, `:91`). A second request at the
  **same** cursor/version replaces and cancels the first (test
  `streaming_completion_second_request_cancels_first_session`). A request at a
  **different** position/version does **not** cancel an earlier in-flight stream
  via `start_session`; that earlier stream is reclaimed on the next document edit
  by `cancel_for_uri` (didChange/didClose, `stream_session.rs:104`) or
  `cancel_for_uri_version` (older version, `stream_session.rs:118`). Cancellation
  is honored mid-stream (`session.is_cancelled()`).
- While streaming, the request result is JSON `null`; the items arrive via
  progress.

### Proof tests

`crates/perl-lsp-rs/tests/lsp_streaming_completion_tests.rs`:
`streaming_completion_returns_null_and_emits_progress`,
`streaming_completion_progress_has_valid_session_and_sequence`,
`streaming_completion_without_ai_falls_back_to_one_shot`,
`streaming_completion_with_streaming_disabled_falls_back`,
`streaming_completion_without_partial_result_token_falls_back`,
`streaming_completion_second_request_cancels_first_session`,
`streaming_completion_on_closed_doc_returns_null`,
`streaming_completion_missing_params_returns_error`,
`streaming_completion_capability_advertised`,
`streaming_completion_progress_schema_validation`.

### Invariant

`kind` is the stable discriminator `"perlInlineCompletionStream"`; `sessionId` +
`sequence` let a client order/deduplicate chunks; `isFinal` marks the terminal
chunk. Clients that don't pass `partialResultToken` still get a correct one-shot
result.

---

## 7. Non-goals / future migration — NextEdit

`crates/perl-lsp-rs-core/src/providers/inline_completion/next_edit.rs` scaffolds a
**NextEdit** capability (suggesting follow-up edits: `MissingImport`,
`TestAssertionBody`, `CallSiteUpdate`, `RenameOccurrence`). It is intentionally
**gated off** — `DefaultOff`, receipt-only, and never emits editor-visible items
in the current build. Its declared safety policy is
`requires_parse_safety = true`, `deterministic_sources_only = true`,
`ai_source_enabled = false`.

**Contract.** Until a dedicated NextEdit activation lands (flag wiring + tests +
its own contract section), NextEdit MUST NOT emit into the
`textDocument/inlineCompletion` response. Activating it is out of scope for the
inline-completion contract above and requires its own spec.
