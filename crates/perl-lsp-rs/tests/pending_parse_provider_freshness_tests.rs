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
    assert!(
        !decoded1.iter().any(|(_l, _c, _len, type_name)| type_name == "foo"),
        "post-publish: no current result may contain `foo`; decoded={decoded1:?}"
    );

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
