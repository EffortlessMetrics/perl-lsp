//! Provider behavior on the pending-parse generation gap (#3396 PR4).
//!
//! Today parsing is fully synchronous, so `DocumentState::current_parsed()`
//! is never `None` in production -- a published `ParsedSnapshot`'s generation
//! always matches the document's text generation immediately after a
//! `didOpen`/`didChange` completes. A future async parse worker will make it
//! possible for the text generation to run ahead of the last published parse
//! snapshot; `current_parsed()` then returns `None` while `latest_parsed()`
//! still returns the previous (now-stale) generation's snapshot.
//!
//! `LspServer::test_apply_text_change_without_reparse` / `test_publish_parse_for_current_generation`
//! (test-only helpers, #3396 PR4) force and then close this gap
//! deterministically, without adding a real async worker. These tests prove
//! every provider behaves correctly while the gap is open:
//!
//! | Provider | Policy when `current_parsed()` is `None` |
//! |---|---|
//! | Completion | bounded text/syntax fallback (may still answer) |
//! | Hover | no exact semantic answer from the stale AST |
//! | Signature help | no exact answer from stale current-file facts (falls back to the name-only builtin table, never the AST) |
//! | Definition / References | no exact answer from stale current-file facts |
//! | Semantic tokens | no fresh current-generation semantic-token claim |
//! | Rename | FAIL CLOSED (produces no edits) |
//! | Safe-delete | FAIL CLOSED (produces no edits) |
//! | Symbols | current facts only (live text fallback, never the stale AST) |
//! | Call hierarchy | current facts only |
//!
//! The headline test, `sub_foo_to_bar_cross_provider_freshness_canary`, walks
//! all of these against one shared document and one shared edit.
//!
//! Run:
//!     RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs \
//!         --features expose_lsp_test_api \
//!         --test pending_parse_provider_freshness_tests

#![cfg(all(feature = "workspace", feature = "expose_lsp_test_api"))]

use perl_lsp::LspServer;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fresh_server() -> LspServer {
    let server = LspServer::new();
    let _ = server.test_handle_initialize_dispatch(Some(json!({
        "capabilities": {},
        "rootUri": null,
        "workspaceFolders": null
    })));
    server
}

/// Build a fresh server and read the advertised
/// `semanticTokensProvider.legend.tokenTypes` array from the *same*
/// `initialize` response used to construct it -- `initialize` may only be
/// sent once per server, so the legend must be captured at construction
/// time rather than re-queried later. Same "decode through the advertised
/// legend, never a hardcoded index list" discipline as
/// `tests/lsp_semantic_legend_contract_tests.rs`, adapted for the in-process
/// `LspServer` test API (which returns the bare result object, not a
/// JSON-RPC envelope).
fn fresh_server_with_legend() -> TestResult<(LspServer, Vec<String>)> {
    let server = LspServer::new();
    let init = server
        .test_handle_initialize_dispatch(Some(json!({
            "capabilities": {},
            "rootUri": null,
            "workspaceFolders": null
        })))?
        .ok_or("initialize must return a result")?;
    let legend = init
        .pointer("/capabilities/semanticTokensProvider/legend/tokenTypes")
        .and_then(Value::as_array)
        .ok_or("semanticTokensProvider.legend.tokenTypes missing from initialize response")?
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    Ok((server, legend))
}

/// Decode a `textDocument/semanticTokens/full` response's flat `data` array
/// into `(line, col, len, type_name)` tuples via the advertised legend.
fn decode_semantic_tokens(
    response: &Option<Value>,
    legend: &[String],
) -> TestResult<Vec<(u64, u64, u64, String)>> {
    let data = response
        .as_ref()
        .and_then(|v| v.get("data"))
        .and_then(Value::as_array)
        .ok_or("semanticTokens response missing data array")?;

    let mut line = 0u64;
    let mut col = 0u64;
    let mut decoded = Vec::new();
    for chunk in data.chunks(5) {
        let [delta_line, delta_start, length, token_type, _modifiers] = chunk else {
            return Err("semanticTokens data length must be divisible by 5".into());
        };
        let dl = delta_line.as_u64().ok_or("delta_line not u64")?;
        let ds = delta_start.as_u64().ok_or("delta_start not u64")?;
        let len = length.as_u64().ok_or("length not u64")?;
        let type_idx = usize::try_from(token_type.as_u64().ok_or("token_type not u64")?)?;

        line += dl;
        col = if dl == 0 { col + ds } else { ds };

        let type_name =
            legend.get(type_idx).cloned().unwrap_or_else(|| format!("OUT_OF_RANGE({type_idx})"));
        decoded.push((line, col, len, type_name));
    }
    Ok(decoded)
}

/// Serialize any JSON value to a string and check whether it contains a
/// literal substring -- used to assert a provider result never leaks a
/// stale identifier ("foo") as a fresh current-generation fact.
fn json_contains(value: &Option<Value>, needle: &str) -> bool {
    value.as_ref().is_some_and(|v| v.to_string().contains(needle))
}

const BEFORE_TEXT: &str = "sub foo { return 1; }\nfoo();\n";
const AFTER_TEXT: &str = "sub bar { return 1; }\nbar();\n";

/// The headline test: applies the `sub foo -> bar` rename-by-edit while the
/// pending-parse gap is forced open, and proves no provider presents `foo`
/// (the stale fact) or a *fresh* claim about `bar` (the not-yet-parsed fact)
/// during the gap -- then proves everything resolves to `bar` once the
/// gap-closing snapshot is published.
#[test]
fn sub_foo_to_bar_cross_provider_freshness_canary() -> TestResult {
    let (server, legend) = fresh_server_with_legend()?;
    // Deliberately avoids the substrings "foo"/"bar" in the URI itself --
    // the `json_contains(&result, "foo")` assertions below scan the whole
    // serialized JSON response, and a URI containing "foo" would produce a
    // false positive unrelated to whether the *symbol* fact is stale.
    let uri = "file:///pending_parse_canary.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;

    // ── Fresh baseline (generation 0, current_parsed() is Some) ──────────
    let sem0 = server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded0 = decode_semantic_tokens(&sem0, &legend)?;
    assert!(
        decoded0.contains(&(0, 0, 3, "keyword".to_string())),
        "baseline: `sub` must decode as keyword; decoded={decoded0:?}"
    );
    assert!(
        decoded0.contains(&(0, 4, 3, "function".to_string())),
        "baseline: `foo` declaration must decode as function; decoded={decoded0:?}"
    );
    assert!(
        decoded0.contains(&(1, 0, 5, "function".to_string())),
        "baseline: `foo()` call must decode as function; decoded={decoded0:?}"
    );
    assert!(
        !decoded0
            .iter()
            .any(|(line, col, _len, type_name)| *line == 0 && *col == 0 && type_name == "function"),
        "baseline: no function token may start at the `sub` column; decoded={decoded0:?}"
    );

    let refs0 = server.test_handle_references(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "context": { "includeDeclaration": true }
    })))?;
    let refs0_locations = refs0.as_ref().and_then(Value::as_array);
    assert!(
        refs0_locations.is_some_and(|a| !a.is_empty()),
        "baseline: references at the foo() call must resolve to at least one location; got: {refs0:?}"
    );

    // ── Force the pending-parse gap: apply the foo->bar edit but withhold
    //    republication of the parse snapshot ──────────────────────────────
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;
    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "helper must bump the text generation without republishing a snapshot"
    );

    // Semantic tokens: no fresh current-generation claim from the N-1 AST --
    // empty data, not a stale "foo" token set and not a claimed-but-unparsed
    // "bar" token set.
    let sem_gap =
        server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded_gap = decode_semantic_tokens(&sem_gap, &legend)?;
    assert!(
        decoded_gap.is_empty(),
        "gap: semantic tokens must not claim generation N from the stale N-1 AST; decoded={decoded_gap:?}"
    );

    // Definition / References: no exact answer from stale current-file facts.
    let def_gap = server.test_handle_definition(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 }
    })))?;
    let def_gap_empty = match &def_gap {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Array(items)) => items.is_empty(),
        _ => false,
    };
    assert!(
        def_gap_empty,
        "gap: definition must not answer from a stale/absent AST; got: {def_gap:?}"
    );

    let refs_gap = server.test_handle_references(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "context": { "includeDeclaration": true }
    })))?;
    let refs_gap_empty = refs_gap.as_ref().and_then(Value::as_array).is_none_or(|a| a.is_empty());
    assert!(
        refs_gap_empty,
        "gap: references must not answer from a stale/absent AST; got: {refs_gap:?}"
    );
    assert!(
        !json_contains(&refs_gap, "foo"),
        "gap: references result must never leak the stale `foo` fact; got: {refs_gap:?}"
    );

    // Rename: FAIL CLOSED -- no edits.
    let rename_gap = server.test_handle_rename(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "newName": "baz"
    })))?;
    let rename_gap_edit_count = rename_gap
        .as_ref()
        .and_then(|v| v.get("changes"))
        .and_then(Value::as_object)
        .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>())
        .unwrap_or(0);
    assert_eq!(
        rename_gap_edit_count, 0,
        "gap: rename must fail closed (zero edits) rather than rename a stale/absent AST; got: {rename_gap:?}"
    );

    // Signature help: the user-defined-function branch requires the AST
    // (`current_parsed()`), which is unavailable during the gap, so it must
    // never surface the stale `foo` signature; the name-only builtin-function
    // fallback does not recognize `foo`.
    let sig_gap = server.test_handle_signature_help(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 4 }
    })))?;
    assert!(
        !json_contains(&sig_gap, "foo"),
        "gap: signature help must never surface the stale `foo` fact; got: {sig_gap:?}"
    );

    // Completion may still answer, but only from its declared bounded
    // text/syntax fallback -- never a claim requiring the (unavailable) AST.
    let completion_gap = server.test_handle_completion(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 3 }
    })))?;
    assert!(
        completion_gap.is_some(),
        "gap: completion must still return a well-formed (possibly bounded) result"
    );

    // No provider result observed so far may present `foo` as a fresh
    // current-generation fact.
    assert!(!json_contains(&def_gap, "foo"), "gap: definition result must not mention `foo`");
    assert!(!json_contains(&rename_gap, "foo"), "gap: rename result must not mention `foo`");

    // ── Close the gap: publish the generation-1 snapshot ─────────────────
    server.test_publish_parse_for_current_generation(uri)?;
    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "publishing the snapshot must not itself advance the text generation"
    );

    let sem1 = server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded1 = decode_semantic_tokens(&sem1, &legend)?;
    assert!(
        decoded1.contains(&(0, 0, 3, "keyword".to_string())),
        "post-publish: `sub` must decode as keyword; decoded={decoded1:?}"
    );
    assert!(
        decoded1.contains(&(0, 4, 3, "function".to_string())),
        "post-publish: `bar` declaration must decode as function; decoded={decoded1:?}"
    );
    assert!(
        decoded1.contains(&(1, 0, 5, "function".to_string())),
        "post-publish: `bar()` call must decode as function; decoded={decoded1:?}"
    );
    // NOTE: there is deliberately no "`decoded1` must not contain `foo`"
    // assertion here. `decode_semantic_tokens` resolves `type_name` from the
    // LSP semantic-token legend (categories like "function"/"keyword"), not
    // from source identifier text -- the wire format is purely positional
    // (line delta, column delta, length, legend index, modifiers bitmask)
    // and never carries the identifier string at all. `type_name == "foo"`
    // would be vacuously false forever regardless of whether a stale `foo`
    // fact leaked, so it cannot detect that. The `refs1` / `json_contains`
    // check right below is the one that actually carries text and proves
    // freshness.

    let refs1 = server.test_handle_references(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "context": { "includeDeclaration": true }
    })))?;
    let refs1_locations = refs1.as_ref().and_then(Value::as_array);
    assert!(
        refs1_locations.is_some_and(|a| !a.is_empty()),
        "post-publish: references must resolve `bar`; got: {refs1:?}"
    );
    assert!(
        !json_contains(&refs1, "foo"),
        "post-publish: references result must not mention `foo`"
    );

    let sig1 = server.test_handle_signature_help(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 4 }
    })))?;
    assert!(
        json_contains(&sig1, "bar"),
        "post-publish: signature help must resolve `bar` once the generation-1 \
         snapshot is current; got: {sig1:?}"
    );
    assert!(
        !json_contains(&sig1, "foo"),
        "post-publish: signature help result must not mention `foo`; got: {sig1:?}"
    );

    Ok(())
}

// ── Individual provider assertions (forced gap, isolated fixtures) ───────

/// Hover: falls back to the text-based token hover (never consults the
/// stale AST) when `current_parsed()` is `None` -- already correct via the
/// #3579 migration to `current_parsed()`. This asserts the handler still
/// answers without error and without claiming AST-derived semantic detail.
#[test]
fn hover_degrades_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_hover.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let hover = server.test_handle_hover(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 }
    })))?;
    // The text-based fallback may or may not produce a hover card for a bare
    // identifier -- the contract under test is "no error, no stale claim",
    // not "always answers".
    assert!(
        !json_contains(&hover, "foo"),
        "gap: hover must never surface the stale `foo` fact; got: {hover:?}"
    );

    Ok(())
}

/// Signature help: falls back to the name-only builtin-function table (never
/// consults the stale AST) when `current_parsed()` is `None` --
/// `get_user_function_signature` requires `doc.current_parsed().ast()`, so
/// that branch is skipped entirely during the gap. Mirrors
/// `hover_degrades_during_pending_parse_gap`'s "no error, no stale claim"
/// contract: `foo` is not a Perl builtin, so the fallback does not know it
/// either.
#[test]
fn signature_help_no_stale_claim_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_signature_help.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let sig = server.test_handle_signature_help(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 4 }
    })))?;
    assert!(
        !json_contains(&sig, "foo"),
        "gap: signature help must never surface the stale `foo` fact; got: {sig:?}"
    );

    Ok(())
}

/// Signature help must never answer from the stale AST even when the
/// function *name* is unchanged across the gap-opening edit.
///
/// `signature_help_no_stale_claim_during_pending_parse_gap` (above) and the
/// headline canary both rename the function (`foo` -> `bar`), so a
/// regression that swapped `current_parsed()` for `latest_parsed()` (reading
/// the stale N-1 AST instead of honestly reporting no fresh answer) would
/// look up `bar` in an AST that only defines `foo` -- the lookup misses by
/// name regardless of which snapshot is consulted, and the assertion passes
/// either way. That makes those tests unable to distinguish "gap handled
/// honestly" from "gap handled by silently falling back to the stale AST"
/// for a same-named function. This test closes that gap: the function name
/// (`calc`) is stable across the edit, only its signature changes, so a
/// stale-AST answer is name-matchable but observably wrong (0 parameters)
/// versus the honest "no answer" the gap policy requires.
#[test]
fn signature_help_never_answers_from_stale_ast_with_matching_name_during_pending_parse_gap()
-> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_signature_help_same_name.pl";

    const BEFORE: &str = "sub calc { return 1; }\ncalc();\n";
    const AFTER: &str = "sub calc($x, $y) { return $x + $y; }\ncalc(1, 2);\n";

    server.test_apply_did_open(uri, BEFORE, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER, 2)?;
    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "helper must bump the text generation without republishing a snapshot"
    );

    let sig = server.test_handle_signature_help(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 5 }
    })))?;

    // The honest gap answer has no user-defined-function signature at all:
    // the AST-gated branch (`get_user_function_signature`) is skipped
    // because `current_parsed()` is `None`, and `calc` is not a Perl
    // builtin. A regression that reads `latest_parsed()` instead would
    // match `calc` by name in the stale (0-parameter) AST and return the
    // label `"sub calc"` -- distinguishably wrong once the live text
    // defines a 2-parameter `calc`.
    let stale_zero_param_label_present =
        sig.as_ref().and_then(|v| v.get("signatures")).and_then(Value::as_array).is_some_and(
            |sigs| sigs.iter().any(|s| s.get("label").and_then(Value::as_str) == Some("sub calc")),
        );
    assert!(
        !stale_zero_param_label_present,
        "gap: signature help must never surface the stale 0-parameter `calc` \
         signature from the N-1 AST just because the name still matches; got: {sig:?}"
    );

    // Post-publish: once the gap-closing snapshot is published, the fresh
    // *2-parameter* `calc` signature must resolve -- proving the honest "no
    // answer" above was a gap-time policy, not a provider that can never
    // find `calc` at all. Checking `parameters.len() == 2` (not merely that
    // the response mentions "calc") is deliberate: a 0-parameter match would
    // also contain the substring "calc" and would silently pass a substring
    // check, undermining the very asymmetry this assertion exists to prove.
    server.test_publish_parse_for_current_generation(uri)?;
    let sig_fresh = server.test_handle_signature_help(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 5 }
    })))?;
    let fresh_param_count = sig_fresh
        .as_ref()
        .and_then(|v| v.get("signatures"))
        .and_then(Value::as_array)
        .and_then(|sigs| sigs.first())
        .and_then(|s| s.get("parameters"))
        .and_then(Value::as_array)
        .map(Vec::len);
    assert_eq!(
        fresh_param_count,
        Some(2),
        "post-publish: signature help must resolve the fresh 2-parameter `calc` \
         signature once the generation-1 snapshot is current, not a 0-parameter \
         (stale-shaped) or missing signature; got: {sig_fresh:?}"
    );

    Ok(())
}

/// Definition: no exact answer from stale current-file facts during the gap.
#[test]
fn definition_fails_closed_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_definition.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let def = server.test_handle_definition(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 }
    })))?;
    let empty = match &def {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Array(items)) => items.is_empty(),
        _ => false,
    };
    assert!(
        empty,
        "gap: definition must not answer from stale/absent current-file facts; got: {def:?}"
    );

    Ok(())
}

/// References: no exact answer from stale current-file facts during the gap.
#[test]
fn references_fail_closed_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_references.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let refs = server.test_handle_references(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "context": { "includeDeclaration": true }
    })))?;
    let empty = refs.as_ref().and_then(Value::as_array).is_none_or(|a| a.is_empty());
    assert!(
        empty,
        "gap: references must not answer from stale/absent current-file facts; got: {refs:?}"
    );

    Ok(())
}

/// Semantic tokens: no fresh current-generation claim from the N-1 AST.
#[test]
fn semantic_tokens_emit_nothing_during_pending_parse_gap() -> TestResult {
    let (server, legend) = fresh_server_with_legend()?;
    let uri = "file:///gap_semantic_tokens.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let full = server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded_full = decode_semantic_tokens(&full, &legend)?;
    assert!(
        decoded_full.is_empty(),
        "gap: semanticTokens/full must not claim generation N from the stale N-1 AST; decoded={decoded_full:?}"
    );

    let range = server.test_handle_semantic_tokens_range(Some(json!({
        "textDocument": { "uri": uri },
        "range": {
            "start": { "line": 0, "character": 0 },
            "end": { "line": 1, "character": 6 }
        }
    })))?;
    let range_data = range.as_ref().and_then(|v| v.get("data")).and_then(Value::as_array);
    assert!(
        range_data.is_none_or(|a| a.is_empty()),
        "gap: semanticTokens/range must not claim generation N from the stale N-1 AST; got: {range:?}"
    );

    Ok(())
}

/// Rename: FAIL CLOSED (zero edits) during the gap.
#[test]
fn rename_fails_closed_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_rename.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let rename = server.test_handle_rename(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "newName": "baz"
    })))?;
    let edit_count = rename
        .as_ref()
        .and_then(|v| v.get("changes"))
        .and_then(Value::as_object)
        .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>())
        .unwrap_or(0);
    assert_eq!(edit_count, 0, "gap: rename must fail closed; got: {rename:?}");

    Ok(())
}

/// Safe-delete: FAIL CLOSED (no live edits) during the gap.
///
/// `refactor_runtime_symbol` -- the shared symbol-at-position lookup behind
/// both the preview and live-pilot safe-delete entry points -- reads
/// `current_parsed()` and returns `None` on the gap, which blocks every
/// downstream guard. This asserts that already-safe behavior via the
/// receipt's `live_provider_result` (a constant `{"changes": {}}` shape) and
/// `decision` fields.
#[test]
fn safe_delete_fails_closed_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_safe_delete.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let receipt = server.test_safe_delete_runtime_blocker_ux_receipt(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 0, "character": 4 }
    })))?;
    let live_edit_count = receipt
        .as_ref()
        .and_then(|v| v.get("live_provider_result"))
        .and_then(|v| v.get("changes"))
        .and_then(Value::as_object)
        .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>())
        .unwrap_or(0);
    assert_eq!(
        live_edit_count, 0,
        "gap: safe-delete must never report live edits; got: {receipt:?}"
    );
    assert_eq!(
        receipt.as_ref().and_then(|v| v.get("decision")).and_then(Value::as_str),
        Some("fallback"),
        "gap: safe-delete blocker receipt must record a fallback decision; got: {receipt:?}"
    );

    Ok(())
}

/// Symbols: current facts only. `handle_document_symbol`'s AST-less branch
/// falls back to a *text*-based regex extractor over the live (already
/// edited) `doc.text` -- not the stale AST -- so it is stale-tolerant only
/// in the sense that its precision is degraded, never in the sense that it
/// can report a superseded identifier.
///
/// // stale-tolerant: never -- the fallback always regex-scans the current
/// // `doc.text`, so it cannot present `foo` once the document has moved to
/// // `bar`, even though `current_parsed()` is `None`.
#[test]
fn document_symbols_never_leak_stale_identifier_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_symbols.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let symbols = server.test_handle_document_symbols(Some(json!({
        "textDocument": { "uri": uri }
    })))?;
    assert!(
        !json_contains(&symbols, "foo"),
        "gap: document symbols must never report the superseded `foo` identifier; got: {symbols:?}"
    );

    Ok(())
}

/// Call hierarchy: current facts only -- `handle_prepare_call_hierarchy`
/// already gates on `current_parsed()` and returns `null` when unavailable.
#[test]
fn prepare_call_hierarchy_fails_closed_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_call_hierarchy.pl";
    server.test_enable_call_hierarchy();

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let hierarchy = server.test_handle_prepare_call_hierarchy(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 }
    })))?;
    let empty = match &hierarchy {
        None => true,
        Some(Value::Null) => true,
        Some(Value::Array(items)) => items.is_empty(),
        _ => false,
    };
    assert!(
        empty,
        "gap: call hierarchy must not answer from stale/absent current-file facts; got: {hierarchy:?}"
    );

    Ok(())
}

/// Completion: the declared bounded text/syntax fallback may still answer
/// during the gap -- unlike the fail-closed providers above, this is the one
/// documented exception in the pending-parse policy table.
#[test]
fn completion_uses_bounded_fallback_during_pending_parse_gap() -> TestResult {
    let server = fresh_server();
    let uri = "file:///gap_completion.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    server.test_apply_text_change_without_reparse(uri, AFTER_TEXT, 2)?;

    let completion = server.test_handle_completion(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 3 }
    })))?;
    assert!(
        completion.is_some(),
        "gap: completion must still return a well-formed result from its bounded fallback"
    );
    assert!(
        !json_contains(&completion, "foo"),
        "gap: completion's bounded fallback must not surface the stale `foo` identifier \
         from any cached AST-derived source; got: {completion:?}"
    );

    Ok(())
}

/// Preserve today's behavior: in the NO-gap case (normal synchronous flow),
/// every provider exercised above must behave exactly as before -- fresh
/// answers, not suppressed. This guards against the new gap-handling logic
/// accidentally firing when there is no gap.
#[test]
fn providers_answer_normally_with_no_pending_parse_gap() -> TestResult {
    let (server, legend) = fresh_server_with_legend()?;
    let uri = "file:///no_gap_baseline.pl";

    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    assert_eq!(server.test_document_generation(uri), Some(0));

    let sem = server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded = decode_semantic_tokens(&sem, &legend)?;
    assert!(
        decoded.contains(&(1, 0, 5, "function".to_string())),
        "no gap: semantic tokens must answer fresh; decoded={decoded:?}"
    );

    let refs = server.test_handle_references(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "context": { "includeDeclaration": true }
    })))?;
    assert!(
        refs.as_ref().and_then(Value::as_array).is_some_and(|a| !a.is_empty()),
        "no gap: references must answer fresh; got: {refs:?}"
    );

    let def = server.test_handle_definition(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 }
    })))?;
    let def_non_empty = match &def {
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Object(_)) => true,
        _ => false,
    };
    assert!(def_non_empty, "no gap: definition must answer fresh; got: {def:?}");

    Ok(())
}

// ── Real async-worker variant (#3396 Phase 3) ─────────────────────────────
//
// Everything above forces the pending-parse gap via the #3589 test-only
// helpers (`test_apply_text_change_without_reparse` /
// `test_publish_parse_for_current_generation`) -- deliberately kept intact
// above, since they remain a valid, cheap proof for provider policy in
// isolation. This test instead installs the REAL off-lock parse worker
// (#3396 Phase 3) and drives the exact same `sub foo -> bar` scenario
// through a genuine asynchronous gap using the worker's pause/release
// barrier, proving the real production wiring -- not just the synthetic
// gap -- produces the same freshness guarantees.

/// Build a fresh, `Arc`-wrapped server with the real parse worker installed
/// and capture the semantic-token legend from the same `initialize`
/// response used to construct it (mirrors `fresh_server_with_legend`, but
/// `Arc`-wrapped so `test_install_parse_worker` can be called).
fn fresh_server_with_real_worker_and_legend() -> TestResult<(Arc<LspServer>, Vec<String>)> {
    let server = Arc::new(LspServer::new());
    server.test_install_parse_worker();
    assert!(
        server.test_parse_worker_installed(),
        "parse worker must report installed immediately after test_install_parse_worker"
    );
    let init = server
        .test_handle_initialize_dispatch(Some(json!({
            "capabilities": {},
            "rootUri": null,
            "workspaceFolders": null
        })))?
        .ok_or("initialize must return a result")?;
    let legend = init
        .pointer("/capabilities/semanticTokensProvider/legend/tokenTypes")
        .and_then(Value::as_array)
        .ok_or("semanticTokensProvider.legend.tokenTypes missing from initialize response")?
        .iter()
        .map(|v| v.as_str().unwrap_or("").to_string())
        .collect();
    Ok((server, legend))
}

/// `sub_foo_to_bar_cross_provider_freshness_canary`, re-run against the REAL
/// off-lock async parse worker instead of the forced test-only gap.
///
/// Sequence: apply the `foo -> bar` edit through the genuine async
/// `didChange` path (worker installed, so the enqueue-and-return path is
/// live), arm the worker's barrier so it pauses immediately before
/// publishing the edit's generation, assert providers see the gap exactly
/// like the synthetic-gap canary does, release the worker, and assert
/// everything resolves to `bar` once the real publish lands.
#[test]
fn sub_foo_to_bar_cross_provider_freshness_canary_real_async_worker() -> TestResult {
    let (server, legend) = fresh_server_with_real_worker_and_legend()?;
    let uri = "file:///pending_parse_canary_real_async.pl";

    // didOpen is unaffected by this PR (always synchronous) -- generation 0.
    server.test_apply_did_open(uri, BEFORE_TEXT, 1)?;
    assert_eq!(server.test_document_generation(uri), Some(0));

    // ── Fresh baseline (generation 0) ─────────────────────────────────────
    let sem0 = server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded0 = decode_semantic_tokens(&sem0, &legend)?;
    assert!(
        decoded0.contains(&(0, 4, 3, "function".to_string())),
        "baseline: `foo` declaration must decode as function; decoded={decoded0:?}"
    );

    // ── Arm the barrier, then apply the foo->bar edit through the REAL
    //    async path. `didChange` must return immediately (enqueue-only) --
    //    the barrier proves the worker got as far as parsing but has not
    //    yet published, by blocking on it explicitly rather than trusting
    //    that `test_apply_did_change` returning fast means anything on its
    //    own (a synchronous fallback would also return "fast" for a tiny
    //    fixture).
    server.test_parse_worker_arm_barrier(uri, 1);
    server.test_apply_did_change(uri, AFTER_TEXT, 2)?;
    server.test_parse_worker_wait_until_paused();

    assert_eq!(
        server.test_document_generation(uri),
        Some(1),
        "the real didChange path must still bump the text generation before returning"
    );

    // ── While genuinely paused mid-publish: providers must show the same
    //    gap behavior as the synthetic-gap canary -- no stale `foo`, no
    //    unearned fresh `bar` claim.
    let sem_gap =
        server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded_gap = decode_semantic_tokens(&sem_gap, &legend)?;
    assert!(
        decoded_gap.is_empty(),
        "real gap: semantic tokens must not claim generation N from the stale N-1 AST; decoded={decoded_gap:?}"
    );

    let refs_gap = server.test_handle_references(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "context": { "includeDeclaration": true }
    })))?;
    let refs_gap_empty = refs_gap.as_ref().and_then(Value::as_array).is_none_or(|a| a.is_empty());
    assert!(
        refs_gap_empty,
        "real gap: references must not answer from a stale/absent AST; got: {refs_gap:?}"
    );
    assert!(
        !json_contains(&refs_gap, "foo"),
        "real gap: references result must never leak the stale `foo` fact; got: {refs_gap:?}"
    );

    let rename_gap = server.test_handle_rename(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "newName": "baz"
    })))?;
    let rename_gap_edit_count = rename_gap
        .as_ref()
        .and_then(|v| v.get("changes"))
        .and_then(Value::as_object)
        .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>())
        .unwrap_or(0);
    assert_eq!(
        rename_gap_edit_count, 0,
        "real gap: rename must fail closed (zero edits); got: {rename_gap:?}"
    );

    // ── Release the worker -- it publishes for real this time.
    server.test_parse_worker_release_barrier();
    assert!(
        server.test_wait_for_parse_worker_settled(uri, Duration::from_secs(5)),
        "worker must settle (publish) within the timeout after release"
    );

    let metrics = server.test_parse_worker_metrics().ok_or("parse worker metrics missing")?;
    assert_eq!(metrics.jobs_published, 1, "exactly one generation must have published");
    // The structural "a rejected/coalesced job never triggers side effects"
    // invariant is proven precisely by
    // `runtime::parse_worker::tests::rejected_publish_never_invokes_the_side_effect_callback`
    // (a counting `on_published` stub, checked against `jobs_rejected_stale`
    // directly). This integration test instead proves the REAL production
    // wiring end-to-end: the provider-facing assertions above and below are
    // the externally observable half of that same invariant.

    let sem1 = server.test_handle_semantic_tokens(Some(json!({"textDocument": {"uri": uri}})))?;
    let decoded1 = decode_semantic_tokens(&sem1, &legend)?;
    assert!(
        decoded1.contains(&(0, 4, 3, "function".to_string())),
        "post-publish: `bar` declaration must decode as function; decoded={decoded1:?}"
    );
    // See the analogous NOTE in `sub_foo_to_bar_cross_provider_freshness_canary`:
    // `decode_semantic_tokens`'s `type_name` comes from the legend (token
    // categories), never from source identifier text, so a
    // `type_name == "foo"` check is vacuously false forever and cannot
    // detect a leaked stale `foo` fact -- the `refs1` / `json_contains`
    // check right below is the one that actually carries text.

    let refs1 = server.test_handle_references(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "context": { "includeDeclaration": true }
    })))?;
    let refs1_locations = refs1.as_ref().and_then(Value::as_array);
    assert!(
        refs1_locations.is_some_and(|a| !a.is_empty()),
        "post-publish: references must resolve `bar`; got: {refs1:?}"
    );
    assert!(
        !json_contains(&refs1, "foo"),
        "post-publish: references result must not mention `foo`"
    );

    let rename1 = server.test_handle_rename(Some(json!({
        "textDocument": { "uri": uri },
        "position": { "line": 1, "character": 0 },
        "newName": "baz"
    })))?;
    let rename1_edit_count = rename1
        .as_ref()
        .and_then(|v| v.get("changes"))
        .and_then(Value::as_object)
        .map(|changes| changes.values().filter_map(Value::as_array).map(Vec::len).sum::<usize>())
        .unwrap_or(0);
    assert!(
        rename1_edit_count > 0,
        "post-publish: rename must succeed once the real generation-1 snapshot is current; got: {rename1:?}"
    );

    Ok(())
}
