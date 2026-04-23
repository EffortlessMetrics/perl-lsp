//! Cross-editor completion conformance tests.
//!
//! These tests emulate completion handshake/request shapes used by
//! VS Code, Zed, Neovim, and Helix so we can lock down parity in
//! snippet formatting and commit character behavior.

mod support;

use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
enum CompletionContextShape {
    Missing,
    Invoked,
    Triggered(char),
}

#[derive(Clone, Copy)]
struct EditorProfile {
    name: &'static str,
    snippet_support: bool,
    context_shape: CompletionContextShape,
}

fn editor_profiles() -> [EditorProfile; 4] {
    [
        EditorProfile {
            name: "vscode",
            snippet_support: true,
            context_shape: CompletionContextShape::Invoked,
        },
        EditorProfile {
            name: "zed",
            snippet_support: true,
            context_shape: CompletionContextShape::Triggered('.'),
        },
        EditorProfile {
            name: "neovim",
            snippet_support: false,
            context_shape: CompletionContextShape::Missing,
        },
        EditorProfile {
            name: "helix",
            snippet_support: false,
            context_shape: CompletionContextShape::Invoked,
        },
    ]
}

fn capabilities_for(profile: EditorProfile) -> Value {
    json!({
        "textDocument": {
            "completion": {
                "completionItem": {
                    "snippetSupport": profile.snippet_support
                }
            }
        }
    })
}

fn completion_request(
    uri: &str,
    line: u32,
    character: u32,
    shape: CompletionContextShape,
) -> Value {
    let mut request = json!({
        "textDocument": { "uri": uri },
        "position": { "line": line, "character": character },
    });

    match shape {
        CompletionContextShape::Missing => {}
        CompletionContextShape::Invoked => {
            request["context"] = json!({"triggerKind": 1});
        }
        CompletionContextShape::Triggered(ch) => {
            request["context"] = json!({
                "triggerKind": 2,
                "triggerCharacter": ch.to_string(),
            });
        }
    }

    request
}

fn completion_items(response: Value) -> Result<Vec<Value>, String> {
    if let Some(items) = response.get("items").and_then(Value::as_array) {
        return Ok(items.to_vec());
    }

    if let Some(items) = response.as_array() {
        return Ok(items.to_vec());
    }

    Err(format!("expected completion array/list, got: {response}"))
}

fn completion_item_by_label<'a>(items: &'a [Value], label: &str) -> Result<&'a Value, String> {
    items
        .iter()
        .find(|item| item.get("label").and_then(Value::as_str) == Some(label))
        .ok_or_else(|| format!("missing completion item '{label}'"))
}

#[test]
fn snippet_insert_text_format_matches_client_capabilities() -> TestResult {
    let uri = "file:///cross_editor_snippets.pl";

    for profile in editor_profiles() {
        let mut harness = LspHarness::new();
        harness.initialize(Some(capabilities_for(profile)))?;
        harness.open(uri, "ife")?;

        let response = harness.request(
            "textDocument/completion",
            completion_request(uri, 0, 3, profile.context_shape),
        )?;

        let items = completion_items(response)?;
        let ife = completion_item_by_label(&items, "ife")?;

        let insert_text = ife
            .get("insertText")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{}: missing insertText", profile.name))?;

        if profile.snippet_support {
            assert!(
                insert_text.contains("${"),
                "{} should receive snippet placeholders, got: {insert_text}",
                profile.name
            );

            if let Some(insert_text_format) = ife.get("insertTextFormat").and_then(Value::as_u64) {
                assert_eq!(
                    insert_text_format, 2,
                    "{} should use snippet insertTextFormat when present",
                    profile.name
                );
            }
        } else {
            assert!(
                !insert_text.contains("${"),
                "{} should receive snippet placeholders degraded to plaintext, got: {insert_text}",
                profile.name
            );

            if let Some(insert_text_format) = ife.get("insertTextFormat").and_then(Value::as_u64) {
                assert_eq!(
                    insert_text_format, 1,
                    "{} should use plain-text insertTextFormat when present",
                    profile.name
                );
            }
        }
    }

    Ok(())
}

#[test]
fn function_commit_characters_are_stable_across_editor_profiles() -> TestResult {
    let uri = "file:///cross_editor_commit_chars.pl";
    let mut baseline: Option<Vec<String>> = None;

    for profile in editor_profiles() {
        let mut harness = LspHarness::new();
        harness.initialize(Some(capabilities_for(profile)))?;
        harness.open(uri, "pri")?;

        let response = harness.request(
            "textDocument/completion",
            completion_request(uri, 0, 3, profile.context_shape),
        )?;

        let items = completion_items(response)?;
        let print_item = completion_item_by_label(&items, "print")?;
        let commit_chars = print_item
            .get("commitCharacters")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{}: print missing commitCharacters", profile.name))?
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();

        assert!(
            commit_chars.iter().any(|ch| ch == "("),
            "{} profile should include '(' commit character for functions",
            profile.name
        );
        assert!(
            commit_chars.iter().any(|ch| ch == ";"),
            "{} profile should include ';' commit character for functions",
            profile.name
        );

        if let Some(expected) = &baseline {
            assert_eq!(
                &commit_chars, expected,
                "{} profile diverged from baseline commit characters",
                profile.name
            );
        } else {
            baseline = Some(commit_chars);
        }
    }

    Ok(())
}
