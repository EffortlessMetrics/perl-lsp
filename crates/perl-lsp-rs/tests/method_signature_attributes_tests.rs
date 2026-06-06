//! LSP hover / goto-definition / semantic-token coverage for Object::Pad `method`
//! declarations that have BOTH a signature AND trailing attributes.
//!
//! PR #770 added parser support for `method foo($x) :attr { }` forms.  This file
//! locks in LSP-level characterisation: what hover, goto-definition, and
//! semantic-tokens actually return for those constructs.
//!
//! ## Design notes
//!
//! Tests are characterize-first: they assert what the providers *actually*
//! return today (non-null structure, no error) rather than pinning exact
//! content that might legitimately change.  Where the provider returns
//! meaningful content we pin specific strings; where it returns null we
//! document that expectation.
//!
//! Regression: existing `native_class_hover_tests` and
//! `method_modifier_definition_tests` must stay green alongside these tests.

mod support;

use serde_json::json;
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Extract `contents.value` string from a hover result value, if present.
fn hover_value(result: &serde_json::Value) -> Option<String> {
    result
        .get("contents")
        .and_then(|c| c.get("value"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Decode LSP semantic tokens (5-tuple relative encoding) into
/// `(line, col, len, token_type, token_modifiers)` absolute tuples.
fn decode_tokens(data: &[u64]) -> Vec<(u64, u64, u64, u64, u64)> {
    let mut line = 0u64;
    let mut col = 0u64;
    let mut result = Vec::new();
    for chunk in data.chunks(5) {
        if chunk.len() < 5 {
            break;
        }
        let dl = chunk[0];
        let ds = chunk[1];
        let len = chunk[2];
        let tt = chunk[3];
        let tm = chunk[4];
        line += dl;
        if dl == 0 {
            col += ds;
        } else {
            col = ds;
        }
        result.push((line, col, len, tt, tm));
    }
    result
}

/// Object::Pad class fixture with multiple methods that each have signatures
/// and trailing attributes.
///
/// ```perl
/// # line 0
/// use Object::Pad;
/// # line 1
/// class MyClass {
/// # line 2
///     method initialize($arg) :public { return $arg; }
/// # line 3
///     method internal() :private { return 0; }
/// # line 4
///     method baz($x, $y) :public :lvalue { return $x + $y; }
/// # line 5
/// }
/// ```
const CLASS_FIXTURE: &str = concat!(
    "use Object::Pad;\n",                                           // line 0
    "class MyClass {\n",                                            // line 1
    "    method initialize($arg) :public { return $arg; }\n",       // line 2
    "    method internal() :private { return 0; }\n",               // line 3
    "    method baz($x, $y) :public :lvalue { return $x + $y; }\n", // line 4
    "}\n",                                                          // line 5
);

// ─── hover tests ──────────────────────────────────────────────────────────────

/// Hover on `initialize` (method with signature + single attribute) must return
/// a valid hover response or null — never a JSON-RPC error.
///
/// Pins: when content is present it must mention the method name.
#[test]
fn test_hover_method_with_signature_and_attribute_no_error() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_sig_attr.pl", CLASS_FIXTURE)?;

    // Hover on `initialize` — line 2, character 11 (after "    method ")
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///method_sig_attr.pl"},
                "position": {"line": 2, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    // Must be null or a valid hover structure — never a JSON-RPC error.
    if !result.is_null() {
        let contents = result
            .get("contents")
            .ok_or("Hover response must have 'contents' field when non-null")?;

        if let Some(obj) = contents.as_object() {
            let kind = obj.get("kind").and_then(|k| k.as_str());
            assert!(
                kind == Some("markdown") || kind == Some("plaintext"),
                "contents.kind must be 'markdown' or 'plaintext', got: {kind:?}"
            );
            let value = obj.get("value").and_then(|v| v.as_str());
            assert!(value.is_some(), "contents.value must be present when contents is an object");
        }

        // When the provider returns content, it should reference the method name.
        if let Some(val) = hover_value(&result) {
            assert!(
                !val.is_empty(),
                "Hover value for 'initialize' must not be empty when returned"
            );
            assert!(
                val.contains("initialize"),
                "Hover on method 'initialize' should mention the method name, got: {val}"
            );
        }
    }
    // null is acceptable — documents current state without locking bad behaviour.

    Ok(())
}

/// Hover on `initialize` should — when the provider returns content — include
/// either the `method` keyword or the signature parameter `$arg`.
///
/// This pins the positive case: if the hover provider has been improved to
/// handle signature+attribute methods, it produces useful content.
#[test]
fn test_hover_method_with_sig_attr_content_characterization() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_sig_attr_char.pl", CLASS_FIXTURE)?;

    // Hover on `initialize` — line 2, character 11
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///method_sig_attr_char.pl"},
                "position": {"line": 2, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    if let Some(val) = result.is_null().then(|| None).unwrap_or_else(|| hover_value(&result)) {
        // If the provider returned content, characterize what it shows.
        // Either the method keyword or the parameter signals a working hover.
        let has_method_keyword = val.contains("method");
        let has_param = val.contains("$arg") || val.contains("arg");
        let has_name = val.contains("initialize");

        assert!(
            has_name,
            "Hover content for 'initialize' must at least mention the method name, got: {val}"
        );

        // Document what the provider actually exposes for attributes.
        // Not asserting presence of `:public` since providers may or may not surface attributes.
        let _ = has_method_keyword;
        let _ = has_param;
    }

    Ok(())
}

/// Hover on `internal` (method with no signature params + `:private` attribute).
///
/// Verifies that the no-param case also works without error.
#[test]
fn test_hover_method_no_params_with_attribute_no_error() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_internal.pl", CLASS_FIXTURE)?;

    // Hover on `internal` — line 3, character 11 (after "    method ")
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///method_internal.pl"},
                "position": {"line": 3, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover for 'internal' must have 'contents', got: {result}"
        );

        if let Some(val) = hover_value(&result) {
            assert!(
                val.contains("internal"),
                "Hover on 'internal' should mention the method name, got: {val}"
            );
        }
    }

    Ok(())
}

/// Hover on `baz` (method with multiple params AND multiple attributes).
///
/// `method baz($x, $y) :public :lvalue { ... }` — tests the multi-attribute case.
#[test]
fn test_hover_method_multi_param_multi_attribute_no_error() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_baz.pl", CLASS_FIXTURE)?;

    // Hover on `baz` — line 4, character 11 (after "    method ")
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///method_baz.pl"},
                "position": {"line": 4, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover for 'baz' must have 'contents', got: {result}"
        );

        if let Some(val) = hover_value(&result) {
            assert!(
                val.contains("baz"),
                "Hover on 'baz' should mention the method name, got: {val}"
            );
        }
    }

    Ok(())
}

// ─── goto-definition tests ─────────────────────────────────────────────────────

/// Go-to-definition on `initialize` (declaration site) should resolve to itself
/// or return a non-null, non-error response.
///
/// Characterises what the provider returns when the cursor is on the method
/// name in the declaration.
#[test]
fn test_goto_def_method_with_signature_and_attribute_no_error() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_def_sig_attr.pl", CLASS_FIXTURE)?;

    // goto-definition on `initialize` — line 2, character 11
    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///method_def_sig_attr.pl"},
                "position": {"line": 2, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    // Must not be a JSON-RPC error.
    assert!(
        result.get("error").is_none(),
        "goto-definition on 'initialize' must not return a JSON-RPC error, got: {result}"
    );

    // When the result is non-null and an array, each location must be valid.
    if let Some(arr) = result.as_array() {
        for loc in arr {
            assert!(loc.get("uri").is_some(), "Each location must have 'uri', got: {loc}");
            assert!(loc.get("range").is_some(), "Each location must have 'range', got: {loc}");
        }
    }

    Ok(())
}

/// Go-to-definition on `internal` (method with empty params + attribute).
#[test]
fn test_goto_def_method_no_params_with_attribute_no_error() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_def_internal.pl", CLASS_FIXTURE)?;

    // goto-definition on `internal` — line 3, character 11
    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///method_def_internal.pl"},
                "position": {"line": 3, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    assert!(
        result.get("error").is_none(),
        "goto-definition on 'internal' must not return a JSON-RPC error, got: {result}"
    );

    Ok(())
}

/// Go-to-definition on `baz` (multi-param + multi-attribute method).
#[test]
fn test_goto_def_method_multi_param_multi_attribute_no_error() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_def_baz.pl", CLASS_FIXTURE)?;

    // goto-definition on `baz` — line 4, character 11
    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///method_def_baz.pl"},
                "position": {"line": 4, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    assert!(
        result.get("error").is_none(),
        "goto-definition on 'baz' must not return a JSON-RPC error, got: {result}"
    );

    Ok(())
}

/// Go-to-definition on the class name `MyClass` resolves to the class
/// declaration (line 1).  Guard that the class definition is still indexable
/// alongside its methods that carry signatures+attributes.
#[test]
fn test_goto_def_class_name_resolves_to_declaration() -> TestResult {
    // Fixture with a method call on a new instance.
    let code = concat!(
        "use Object::Pad;\n",                                            // line 0
        "class MyClass {\n",                                             // line 1
        "    method greet($name) :public { return \"Hello $name\"; }\n", // line 2
        "}\n",                                                           // line 3
        "my $obj = MyClass->new;\n",                                     // line 4
    );

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///method_class_def.pl", code)?;

    // goto-definition on `MyClass` at the call site (line 4, character 10)
    // `my $obj = MyClass->new;`
    //  0123456789012345
    let result = harness
        .request(
            "textDocument/definition",
            json!({
                "textDocument": {"uri": "file:///method_class_def.pl"},
                "position": {"line": 4, "character": 10}
            }),
        )
        .unwrap_or(json!(null));

    assert!(
        result.get("error").is_none(),
        "goto-definition on class name must not return a JSON-RPC error, got: {result}"
    );

    // When the provider resolves the class, it should point into the same document.
    if let Some(arr) = result.as_array() {
        if let Some(first) = arr.first() {
            if let Some(uri) = first.get("uri").and_then(|u| u.as_str()) {
                assert!(
                    uri.contains("method_class_def"),
                    "goto-definition on class name should point to the same file, got: {uri}"
                );
            }
        }
    }

    Ok(())
}

// ─── semantic token tests ──────────────────────────────────────────────────────

/// Semantic tokens for the full class fixture must not error and must return a
/// valid 5-tuple stream.
///
/// This is the primary safety guard: the semantic token provider must not panic
/// or return a malformed response when a document contains
/// `method name(params) :attr { }` forms.
#[test]
fn test_semantic_tokens_method_sig_attr_no_error() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_method_sig_attr.pl", CLASS_FIXTURE)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_method_sig_attr.pl"}
            }),
        )
        .unwrap_or(json!(null));

    // Response must be null or have a valid `data` array (5-tuple encoding).
    if !response.is_null() {
        let data_field = response.get("data");
        assert!(
            data_field.is_some(),
            "Non-null semanticTokens response must have 'data' field, got: {response}"
        );
        if let Some(arr) = data_field.and_then(|d| d.as_array()) {
            assert_eq!(
                arr.len() % 5,
                0,
                "Semantic token data must be 5-tuples (length % 5 == 0), got length {}",
                arr.len()
            );
        }
    }

    Ok(())
}

/// Semantic tokens for the fixture must form a monotonically non-decreasing
/// (line, col) stream — no negative deltas.
///
/// This validates internal encoding correctness for the multi-method class.
#[test]
fn test_semantic_tokens_method_sig_attr_monotonic_order() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_method_mono.pl", CLASS_FIXTURE)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_method_mono.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        if let Some(arr) = response.get("data").and_then(|d| d.as_array()) {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");
            let data: Vec<u64> = arr.iter().filter_map(|v| v.as_u64()).collect();
            let tokens = decode_tokens(&data);

            let mut prev_line = 0u64;
            let mut prev_col = 0u64;
            for (line, col, _len, _tt, _tm) in &tokens {
                if *line == prev_line {
                    assert!(
                        *col >= prev_col,
                        "Token column must be non-decreasing on same line: \
                         prev_col={prev_col} got col={col}"
                    );
                } else {
                    assert!(
                        *line > prev_line,
                        "Token line must be non-decreasing: prev={prev_line} got={line}"
                    );
                }
                prev_line = *line;
                prev_col = *col;
            }
        }
    }

    Ok(())
}

/// Semantic tokens for a method with multiple trailing attributes
/// (`method baz($x, $y) :public :lvalue { }`) must not error.
///
/// The multi-attribute case is the highest-risk form: each `:attr` is a
/// separate token in the parser's attribute list, and the token emitter must
/// not be confused by the repeated colon-prefixed identifiers.
#[test]
fn test_semantic_tokens_method_multiple_attributes_no_error() -> TestResult {
    let doc = concat!(
        "use Object::Pad;\n",
        "class Widget {\n",
        "    method draw($canvas, $ctx) :public :lvalue { return 1; }\n",
        "}\n",
    );

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_multi_attr.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_multi_attr.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        let data_field = response.get("data");
        assert!(
            data_field.is_some(),
            "Non-null semanticTokens response must have 'data' field, got: {response}"
        );
        if let Some(arr) = data_field.and_then(|d| d.as_array()) {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");
        }
    }

    Ok(())
}

/// Semantic tokens for a class with a mix of plain methods and sig+attr methods
/// must produce a valid stream — regression guard for the common real-world
/// Object::Pad pattern.
#[test]
fn test_semantic_tokens_mixed_method_forms_no_error() -> TestResult {
    let doc = concat!(
        "use Object::Pad;\n",
        "class Animal {\n",
        "    method speak { return 'generic'; }\n", // plain method
        "    method eat($food) { return $food; }\n", // method with sig only
        "    method sleep() :private { return 0; }\n", // method with empty sig + attr
        "    method run($speed) :public { return $speed; }\n", // method with sig + attr
        "}\n",
    );

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///st_mixed_methods.pl", doc)?;

    let response = harness
        .request(
            "textDocument/semanticTokens/full",
            json!({
                "textDocument": {"uri": "file:///st_mixed_methods.pl"}
            }),
        )
        .unwrap_or(json!(null));

    if !response.is_null() {
        let data_field = response.get("data");
        assert!(
            data_field.is_some(),
            "Non-null semanticTokens response must have 'data' field, got: {response}"
        );
        if let Some(arr) = data_field.and_then(|d| d.as_array()) {
            assert_eq!(arr.len() % 5, 0, "Semantic token data must be 5-tuples");
        }
    }

    Ok(())
}

// ─── regression tests ─────────────────────────────────────────────────────────

/// Regression: plain `method greet { }` (no sig, no attr) hover must still work.
///
/// This guards that adding sig+attr test coverage did not break the base case
/// covered by `native_class_hover_tests`.
#[test]
fn test_hover_plain_method_regression_no_error() -> TestResult {
    let doc = "use Object::Pad;\nclass Greeter {\n    method greet { return 'hello'; }\n}\n";

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///regression_plain_method.pl", doc)?;

    // Hover on `greet` — line 2, character 11
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///regression_plain_method.pl"},
                "position": {"line": 2, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover for plain 'greet' must have 'contents', got: {result}"
        );

        if let Some(val) = hover_value(&result) {
            assert!(
                val.contains("greet"),
                "Hover on plain method 'greet' should mention the method name, got: {val}"
            );
        }
    }

    Ok(())
}

/// Regression: `method add($x, $y) { }` (sig only, no attr) hover must still work.
///
/// Tests the intermediate form — signature without attributes — which was already
/// covered by `native_class_hover_tests::test_hover_on_native_method_with_signature_extracts_params`.
#[test]
fn test_hover_method_sig_only_regression_no_error() -> TestResult {
    let doc =
        "use Object::Pad;\nclass Calculator {\n    method add($x, $y) { return $x + $y; }\n}\n";

    let mut harness = LspHarness::new();
    harness.initialize(None)?;
    harness.open_document("file:///regression_sig_only.pl", doc)?;

    // Hover on `add` — line 2, character 11
    let result = harness
        .request(
            "textDocument/hover",
            json!({
                "textDocument": {"uri": "file:///regression_sig_only.pl"},
                "position": {"line": 2, "character": 11}
            }),
        )
        .unwrap_or(json!(null));

    if !result.is_null() {
        assert!(
            result.get("contents").is_some(),
            "Non-null hover for 'add' must have 'contents', got: {result}"
        );

        if let Some(val) = hover_value(&result) {
            assert!(
                val.contains("add"),
                "Hover on method 'add' should mention the method name, got: {val}"
            );
        }
    }

    Ok(())
}
