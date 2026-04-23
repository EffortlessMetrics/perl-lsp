# How to Add a New LSP Feature End-to-End

This guide walks through every step required to ship a new LSP feature in
perl-lsp, from the provider implementation to the capability advertisement, tests,
and catalog entry. It uses a concrete example throughout: adding a new code-action
kind called "Add missing `use strict`".

If you need architectural background first, read
[`docs/reference/LSP_IMPLEMENTATION_GUIDE.md`](../reference/LSP_IMPLEMENTATION_GUIDE.md).
This guide is hands-on — come back to it once you have the codebase checked out
and `cargo test -p perl-lsp-rs` passing.

---

## Overview: the five touch-points

Every new feature requires exactly these changes, in this order:

1. **Provider logic** — implement the business logic in `crates/perl-lsp-rs/src/features/` or a dedicated `perl-lsp-*` crate.
2. **Language handler** — add a method on `LspServer` in `crates/perl-lsp-rs/src/runtime/language/`.
3. **Dispatch wiring** — route the JSON-RPC method string to your handler in `crates/perl-lsp-rs/src/runtime/dispatch/`.
4. **Capability flag** — tell the client you support the feature via `BuildFlags` and `capabilities_for()`.
5. **Catalog entry** — register the feature in `features.toml` and add tests.

---

## Step 1: write the provider

Providers live in `crates/perl-lsp-rs/src/features/`. Simple features can be a single
file; complex ones use a sub-directory.

The existing code-action provider is a good model:

```
crates/perl-lsp-rs/src/features/code_actions_provider/
    mod.rs          ← public struct CodeActionsProvider, get_code_actions()
    fixes.rs        ← concrete fix implementations
    source_utils.rs ← shared text-manipulation helpers
```

For our example (adding a new code-action kind), we extend the existing provider.
For a brand-new feature you would create a parallel structure.

The provider contract is: take parsed inputs (AST, source text, diagnostics),
return domain objects. It must not touch JSON or LSP types. That translation
happens in the handler.

```rust
// crates/perl-lsp-rs/src/features/code_actions_provider/mod.rs (excerpt)

/// Provides code actions (quick-fixes) for diagnostics.
pub struct CodeActionsProvider {
    source: String,
}

impl CodeActionsProvider {
    pub fn new(source: String) -> Self {
        Self { source }
    }

    /// Returns code actions for the given byte-offset range and diagnostics.
    pub fn get_code_actions(
        &self,
        range: (usize, usize),
        diagnostics: &[Diagnostic],
    ) -> Vec<CodeAction> {
        // ... build and return Vec<CodeAction>
    }
}
```

Key conventions:
- No `unwrap()`, `expect()`, or `panic!()` — use `?` or return empty results.
- Return domain types (`CodeAction`, `TextEdit`, …), not `serde_json::Value`.
- Keep the provider `pub` so integration tests can call it directly.

---

## Step 2: add the language handler

Handlers live in `crates/perl-lsp-rs/src/runtime/language/`. One file per feature
group:

```
crates/perl-lsp-rs/src/runtime/language/
    code_actions.rs    ← textDocument/codeAction, codeAction/resolve
    completion.rs      ← textDocument/completion
    hover.rs           ← textDocument/hover
    navigation.rs      ← definition, declaration, type-definition, …
    references.rs      ← textDocument/references
    symbols.rs         ← documentSymbol, workspaceSymbol
    ...
```

Add your method as an `impl LspServer` block in the appropriate file. For a new
code-action kind you would extend `code_actions.rs`; for a brand-new method you
would create a new file and add a `mod` line to
`crates/perl-lsp-rs/src/runtime/language/mod.rs`.

The handler pattern is uniform across all features:

```rust
// crates/perl-lsp-rs/src/runtime/language/code_actions.rs (excerpt)

use super::super::*;
use crate::protocol::{req_range, req_uri};

impl LspServer {
    /// Handle textDocument/codeAction request.
    pub(crate) fn handle_code_action(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        let params = match params {
            Some(p) => p,
            None => return Ok(Some(json!([]))),
        };

        let uri = req_uri(&params)?;
        let ((start_line, start_char), (end_line, end_char)) = req_range(&params)?;

        let documents = self.documents_guard();
        let doc = match self.get_document(&documents, uri) {
            Some(d) => d,
            None => return Ok(Some(json!([]))),
        };

        if let Some(ast) = &doc.ast {
            let start_offset = self.pos16_to_offset(doc, start_line, start_char);
            let end_offset   = self.pos16_to_offset(doc, end_line, end_char);

            // Call the provider
            let provider = CodeActionsProvider::new(doc.text.clone());
            let diag_provider = DiagnosticsProvider::new(ast, doc.text.clone());
            let diagnostics = diag_provider.get_diagnostics(ast, &doc.parse_errors, &doc.text, None);
            let actions = provider.get_code_actions((start_offset, end_offset), &diagnostics);

            // Translate domain types to LSP JSON
            let response: Vec<Value> = actions.iter().map(|a| json!({
                "title": a.title,
                "kind":  code_action_kind_str(&a.kind),
                "edit": {
                    "changes": {
                        uri: [{
                            "range": lsp_range_from_offsets(doc, a.edit.range.0, a.edit.range.1),
                            "newText": a.edit.new_text,
                        }]
                    }
                }
            })).collect();

            return Ok(Some(json!(response)));
        }

        Ok(Some(json!([])))
    }
}
```

Helpers available in `super::super::*` (re-exported from `LspServer`'s impl scope):

| Helper | Purpose |
|--------|---------|
| `self.pos16_to_offset(doc, line, char)` | UTF-16 position to byte offset |
| `self.offset_to_pos16(doc, offset)` | Byte offset to UTF-16 position |
| `self.documents_guard()` | Acquire document map lock |
| `self.get_document(&guard, uri)` | Look up a document by URI |
| `req_uri(&params)` | Extract `textDocument.uri` or return `JsonRpcError` |
| `req_position(&params)` | Extract `position` field |
| `req_range(&params)` | Extract `range` field |

---

## Step 3: wire the dispatch

Open `crates/perl-lsp-rs/src/runtime/dispatch/mod.rs`. There are two places to touch:

### 3a. Register the method for cancellation tracking

Long-running operations (anything that might take >50 ms) should be listed in the
`needs_cancellation` match arm so the server can cancel them if the client sends
`$/cancelRequest`:

```rust
// crates/perl-lsp-rs/src/runtime/dispatch/mod.rs (excerpt)
let needs_cancellation = matches!(
    request.method.as_str(),
    "textDocument/completion"
        | "textDocument/hover"
        | "textDocument/codeAction"   // already present
        // add your method here if it is long-running
        | "textDocument/yourNewMethod"
        | ...
);
```

Lightweight notification handlers that return immediately do not need this.

### 3b. Route the method string to your handler

Add a match arm in the main dispatch function:

```rust
// crates/perl-lsp-rs/src/runtime/dispatch/mod.rs
"textDocument/codeAction"   => self.handle_code_action_dispatch(request.params),
"codeAction/resolve"        => self.handle_code_action_resolve_dispatch(request.params),
// your new method:
"textDocument/yourNewMethod" => self.handle_your_new_method_dispatch(request.params),
```

### 3c. Add the dispatch shim

Dispatch shims live in `crates/perl-lsp-rs/src/runtime/dispatch/text_document.rs`
(for `textDocument/*`) or `workspace.rs` (for `workspace/*`). The shim is a thin
wrapper that calls your language handler:

```rust
// crates/perl-lsp-rs/src/runtime/dispatch/text_document.rs
impl LspServer {
    pub(super) fn handle_your_new_method_dispatch(
        &self,
        params: Option<Value>,
    ) -> Result<Option<Value>, JsonRpcError> {
        self.handle_your_new_method(params)
    }
}
```

The split between dispatch shims and language handlers exists to keep routing
logic separate from business logic — do not put implementation details in the shim.

---

## Step 4: advertise the capability

The server tells the client what it supports during the `initialize` handshake.
This is driven by a chain of three files:

```
crates/perl-lsp-feature-flags/src/lib.rs    ← BuildFlags struct (one bool per feature)
crates/perl-lsp-protocol/src/capabilities.rs ← capabilities_for(BuildFlags) → ServerCapabilities
crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs ← calls capabilities_for(), sends response
```

### 4a. Add a field to `BuildFlags`

`BuildFlags` is in `crates/perl-lsp-feature-flags/src/lib.rs`:

```rust
pub struct BuildFlags {
    // ... existing fields ...
    /// Your new feature compilation flag.
    pub your_new_feature: bool,
}
```

Also add it to the `production()`, `ga_lock()`, and `all()` constructors in
the same file, and to `to_advertised_features()` and `to_feature_ids()` if they
exist.

### 4b. Advertise the capability in `capabilities_for()`

In `crates/perl-lsp-protocol/src/capabilities.rs`, add a block:

```rust
if build.your_new_feature {
    caps.your_capability_provider = Some(/* lsp-types value */);
}
```

The `caps` variable is an `lsp_types::ServerCapabilities`. See the lsp-types 0.97
docs for the exact field name and type.

### 4c. Handle client-side disabling (optional)

If the feature should be disableable via `initializationOptions.disabledFeatures`,
add a case to `apply_disabled_feature_id()` in
`crates/perl-lsp-rs/src/runtime/lifecycle/capabilities.rs`:

```rust
"lsp.your_new_feature" => flags.your_new_feature = false,
```

---

## Step 5: add to `features.toml`

`features.toml` is the canonical capability catalog. Add an entry:

```toml
[[feature]]
id          = "lsp.your_new_feature"
spec        = "LSP 3.17"
area        = "text_document"   # or "workspace"
maturity    = "experimental"    # "experimental" | "beta" | "ga"
advertised  = true
tests       = ["tests/lsp_your_feature_tests.rs"]
description = "One-line description of what this feature does"
```

Once the feature is stable enough for production, change `maturity` to `"ga"`.

---

## Step 6: write tests

### Unit test: provider logic (fast, no server)

```rust
// crates/perl-lsp-rs/tests/your_feature_tests.rs
// (or in the provider module under #[cfg(test)])

use perl_lsp::features::your_feature_provider::YourFeatureProvider;
use perl_parser::Parser;
use std::sync::Arc;

#[test]
fn test_your_feature_basic_case() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        # Perl source that exercises the feature
        my $x = 1;
    "#;

    let mut parser = Parser::new(source);
    let ast = Arc::new(parser.parse()?);

    let provider = YourFeatureProvider::new(source.to_string());
    let results = provider.compute(&ast);

    assert!(!results.is_empty());
    assert_eq!(results[0].title, "Expected title");
    Ok(())
}
```

### Integration test: JSON-RPC over a real server

```rust
// crates/perl-lsp-rs/tests/lsp_your_feature_tests.rs

mod support;
use serde_json::json;
use std::time::Duration;
use support::{LspHarness, TempWorkspace};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn setup_server() -> Result<(LspHarness, TempWorkspace), Box<dyn std::error::Error>> {
    let (mut harness, workspace) = LspHarness::with_workspace(&[
        ("test.pl", "my $x = 1;\nprint $x;\n"),
    ])?;
    harness.open_document(&workspace.uri("test.pl"), "my $x = 1;\nprint $x;\n")?;
    harness.wait_for_idle(Duration::from_millis(500));
    Ok((harness, workspace))
}

#[test]
fn test_your_feature_returns_expected_result() -> TestResult {
    let (mut harness, workspace) = setup_server()?;

    let response = harness.request_with_timeout(
        "textDocument/yourNewMethod",
        json!({
            "textDocument": { "uri": workspace.uri("test.pl") },
            "position": { "line": 0, "character": 4 }
        }),
        Duration::from_secs(5),
    )?;

    assert!(response.is_array() || response.is_object(), "got: {response}");
    // add specific assertions
    Ok(())
}
```

Run tests with threading constraints:

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

---

## Verification checklist

Before opening a PR, run the standard gate:

```bash
export CARGO_TARGET_DIR="/tmp/$(git branch --show-current | tr '/' '-')-target"
cargo fmt --all
cargo clippy --workspace
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2
```

Or run all tiers at once (requires nix):

```bash
nix develop -c just ci-gate
```

Item-by-item checklist:

- [ ] Provider lives in `crates/perl-lsp-rs/src/features/` and has no LSP/JSON imports
- [ ] Language handler is in `crates/perl-lsp-rs/src/runtime/language/`
- [ ] Dispatch shim added to `crates/perl-lsp-rs/src/runtime/dispatch/text_document.rs` (or `workspace.rs`)
- [ ] Method string wired in `crates/perl-lsp-rs/src/runtime/dispatch/mod.rs`
- [ ] `BuildFlags` field added in `crates/perl-lsp-feature-flags/src/lib.rs`
- [ ] Capability advertised in `crates/perl-lsp-protocol/src/capabilities.rs`
- [ ] `features.toml` entry added
- [ ] Unit test covers provider logic
- [ ] Integration test uses `LspHarness` and sends real JSON-RPC
- [ ] No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()` in production code
- [ ] `cargo fmt` and `cargo clippy` are clean

---

## Where to look for real examples

| Feature | Provider | Handler | Dispatch shim | Test |
|---------|----------|---------|---------------|------|
| Code actions | `src/features/code_actions_provider/mod.rs` | `src/runtime/language/code_actions.rs` | `src/runtime/dispatch/text_document.rs:255` | `tests/code_actions_enhanced_tests.rs` |
| Hover | `src/features/` (inline in handler) | `src/runtime/language/hover.rs` | `src/runtime/dispatch/text_document.rs` | `tests/semantic_hover.rs` |
| Completion | `src/features/completion.rs` | `src/runtime/language/completion.rs` | `src/runtime/dispatch/text_document.rs` | `tests/lsp_workspace_completion_tests.rs` |
| References | `src/features/references.rs` | `src/runtime/language/references.rs` | `src/runtime/dispatch/text_document.rs` | `tests/lsp_workspace_index_e2e.rs` |

All paths are relative to `crates/perl-lsp-rs/`.

---

## Related documentation

- [`docs/reference/LSP_IMPLEMENTATION_GUIDE.md`](../reference/LSP_IMPLEMENTATION_GUIDE.md) — architecture overview, client model, generation counters
- [`docs/reference/LSP_TEST_INFRASTRUCTURE.md`](../reference/LSP_TEST_INFRASTRUCTURE.md) — test harness internals, `LspHarness` API
- [`docs/reference/LSP_PROVIDERS_REFERENCE.md`](../reference/LSP_PROVIDERS_REFERENCE.md) — provider catalog with maturity status
- [`docs/reference/POSITION_TRACKING_GUIDE.md`](../reference/POSITION_TRACKING_GUIDE.md) — UTF-16 position handling
- [`features.toml`](../../features.toml) — canonical feature catalog
