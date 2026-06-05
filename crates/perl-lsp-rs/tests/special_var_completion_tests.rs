/// Integration tests for Perl special-variable completion (issue #788)
///
/// Verifies that the LSP server offers perlvar special/magic variables
/// (`$_`, `$!`, `@_`, `%ENV`, etc.) as completion items with documentation
/// when the user types a sigil, and that existing lexical-variable and
/// builtin-function completions are not displaced.
use serde_json::json;
use std::time::Duration;

mod common;
use common::{
    completion_items, drain_until_quiet, initialize_lsp, send_notification, send_request,
    start_lsp_server,
};

// ── helpers ────────────────────────────────────────────────────────────────

/// Collect the `label` strings from a completion response.
fn labels_from(items: &[serde_json::Value]) -> Vec<String> {
    items.iter().filter_map(|item| item["label"].as_str().map(|s| s.to_string())).collect()
}

/// Return the `documentation` field (string form) for the item with the given label.
fn doc_for<'a>(items: &'a [serde_json::Value], label: &str) -> Option<&'a str> {
    items.iter().find(|item| item["label"].as_str() == Some(label)).and_then(|item| {
        // documentation can be a plain string or { kind, value }
        item["documentation"].as_str().or_else(|| item["documentation"]["value"].as_str())
    })
}

// ── scalar special variables ───────────────────────────────────────────────

/// Typing `$` alone triggers scalar special-variable completions with docs.
#[test]
fn test_special_scalar_vars_with_dollar_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test_special.pl";
    let text = "$";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }
        }),
    );
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_secs(2));

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 1 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels = labels_from(items);

    // Core special scalars must be present
    for required in ["$_", "$!", "$/", "$@", "$?", "$$", "$0", "$1"] {
        assert!(
            labels.contains(&required.to_string()),
            "expected {required} in scalar completions; got labels: {labels:?}"
        );
    }

    // Extended capture-group vars added in issue #788
    for required in ["$2", "$3", "$4", "$5", "$6", "$7", "$8", "$9"] {
        assert!(
            labels.contains(&required.to_string()),
            "expected {required} (capture-group var) in scalar completions; got labels: {labels:?}"
        );
    }

    // Additional perlvar scalars added in issue #788
    for required in ["$;", "$\"", "$|", "$^X", "$^I", "$^F"] {
        assert!(
            labels.contains(&required.to_string()),
            "expected {required} in scalar completions; got labels: {labels:?}"
        );
    }

    // Every returned special variable must carry non-empty documentation
    for label in &labels {
        if label.starts_with('$')
            && !label.chars().skip(1).next().is_some_and(char::is_alphanumeric)
        {
            // Looks like a special var (starts with $ followed by non-alphanumeric)
            let doc = doc_for(items, label);
            assert!(
                doc.is_some_and(|d| !d.is_empty()),
                "special variable {label} must have non-empty documentation"
            );
        }
    }

    Ok(())
}

// ── array special variables ────────────────────────────────────────────────

/// Typing `@` alone triggers array special-variable completions with docs.
#[test]
fn test_special_array_vars_with_at_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test_special_array.pl";
    let text = "@";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }
        }),
    );
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_secs(2));

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 1 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels = labels_from(items);

    for required in ["@_", "@ARGV", "@INC", "@ISA", "@EXPORT", "@EXPORT_OK"] {
        assert!(
            labels.contains(&required.to_string()),
            "expected {required} in array completions; got labels: {labels:?}"
        );
        let doc = doc_for(items, required);
        assert!(doc.is_some_and(|d| !d.is_empty()), "{required} must have non-empty documentation");
    }

    Ok(())
}

// ── hash special variables ─────────────────────────────────────────────────

/// Typing `%` alone triggers hash special-variable completions with docs.
#[test]
fn test_special_hash_vars_with_percent_prefix() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test_special_hash.pl";
    let text = "%";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }
        }),
    );
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_secs(2));

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": 0, "character": 1 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels = labels_from(items);

    for required in ["%ENV", "%INC", "%SIG"] {
        assert!(
            labels.contains(&required.to_string()),
            "expected {required} in hash completions; got labels: {labels:?}"
        );
        let doc = doc_for(items, required);
        assert!(doc.is_some_and(|d| !d.is_empty()), "{required} must have non-empty documentation");
    }

    Ok(())
}

// ── regression: lexical variables still offered ────────────────────────────

/// Lexical variables declared in the file still appear alongside special vars.
#[test]
fn test_special_vars_do_not_displace_lexical_vars() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();
    initialize_lsp(&server);

    let uri = "file:///test_regression_lexical.pl";
    // Declare a lexical and then trigger completion with its prefix
    let text = "my $xyzzy = 1;\n$x";
    send_notification(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "perl",
                    "version": 1,
                    "text": text
                }
            }
        }),
    );
    drain_until_quiet(&server, Duration::from_millis(100), Duration::from_secs(2));

    let response = send_request(
        &server,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                // position is end of "$x" on second line
                "position": { "line": 1, "character": 2 }
            }
        }),
    );

    let items = completion_items(&response);
    let labels = labels_from(items);

    assert!(
        labels.contains(&"$xyzzy".to_string()),
        "lexical $xyzzy must appear alongside special variables; got labels: {labels:?}"
    );

    Ok(())
}
