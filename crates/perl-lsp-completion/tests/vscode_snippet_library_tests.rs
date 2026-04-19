//! Validation tests for the VSCode Perl snippet library.
//!
//! These tests verify that `vscode-extension/snippets/perl.json` is:
//!   1. Valid JSON
//!   2. Contains all required snippet prefix categories
//!   3. Meets the minimum count threshold
//!   4. Each snippet has required fields (prefix, body, description)

use std::collections::HashSet;

/// Resolve the path to the snippets file from the repo root.
/// Walks up from CARGO_MANIFEST_DIR to find the directory containing
/// `vscode-extension/snippets/perl.json`.
fn snippet_file_path() -> Result<std::path::PathBuf, String> {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut dir = manifest_dir.as_path();
    loop {
        let candidate = dir
            .join("vscode-extension")
            .join("snippets")
            .join("perl.json");
        if candidate.exists() {
            return Ok(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return Err(
                "walked past filesystem root without finding vscode-extension/snippets/perl.json"
                    .to_string(),
            ),
        }
    }
}

/// Parse the snippet file and return a serde_json Value.
fn load_snippets() -> Result<serde_json::Value, String> {
    let path = snippet_file_path()?;
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&content).map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

/// Collect all prefix values (string or array-of-strings) from the snippet map.
fn all_prefixes(snippets: &serde_json::Value) -> HashSet<String> {
    let mut prefixes = HashSet::new();
    if let Some(map) = snippets.as_object() {
        for (_name, snippet) in map {
            match &snippet["prefix"] {
                serde_json::Value::String(s) => {
                    prefixes.insert(s.clone());
                }
                serde_json::Value::Array(arr) => {
                    for v in arr {
                        if let serde_json::Value::String(s) = v {
                            prefixes.insert(s.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    prefixes
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_snippet_file_is_valid_json() -> Result<(), String> {
    load_snippets().map(|_| ())
}

#[test]
fn test_snippet_count_meets_minimum() -> Result<(), String> {
    let snippets = load_snippets()?;
    let count = snippets.as_object().map(|m| m.len()).unwrap_or(0);
    assert!(count >= 50, "expected at least 50 snippets, found {count}");
    Ok(())
}

#[test]
fn test_snippet_control_flow_coverage() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    let required = ["if", "unless", "while", "until", "for", "foreach"];
    for p in required {
        assert!(
            prefixes.contains(p),
            "missing control-flow snippet prefix: '{p}'"
        );
    }
    Ok(())
}

#[test]
fn test_snippet_do_while_present() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    assert!(
        prefixes.contains("do-while") || prefixes.contains("dowhile"),
        "missing do-while/dowhile snippet prefix"
    );
    Ok(())
}

#[test]
fn test_snippet_anonymous_sub_present() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    assert!(
        prefixes.contains("anon-sub")
            || prefixes.contains("anonsub")
            || prefixes.contains("sub-anon"),
        "missing anonymous sub snippet prefix (anon-sub / anonsub / sub-anon)"
    );
    Ok(())
}

#[test]
fn test_snippet_moose_class_present() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    assert!(
        prefixes.contains("moose-class") || prefixes.contains("mooseclass"),
        "missing Moose class snippet prefix (moose-class / mooseclass)"
    );
    Ok(())
}

#[test]
fn test_snippet_moo_class_present() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    assert!(
        prefixes.contains("moo-class") || prefixes.contains("mooclass"),
        "missing Moo class snippet prefix (moo-class / mooclass)"
    );
    Ok(())
}

#[test]
fn test_snippet_moose_has_attribute_present() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    assert!(
        prefixes.contains("has") || prefixes.contains("moose-has") || prefixes.contains("moo-has"),
        "missing Moose/Moo 'has' attribute snippet"
    );
    Ok(())
}

#[test]
fn test_snippet_qr_regex_present() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    assert!(
        prefixes.contains("qr") || prefixes.contains("qr//"),
        "missing qr// compiled regex snippet"
    );
    Ok(())
}

#[test]
fn test_snippet_deref_patterns_present() -> Result<(), String> {
    let snippets = load_snippets()?;
    let prefixes = all_prefixes(&snippets);
    assert!(
        prefixes.contains("deref-array")
            || prefixes.contains("derefarray")
            || prefixes.contains("deref-hash")
            || prefixes.contains("derefhash")
            || prefixes.contains("deref"),
        "missing dereference snippet (deref-array / deref-hash / deref)"
    );
    Ok(())
}

#[test]
fn test_each_snippet_has_required_fields() -> Result<(), String> {
    let snippets = load_snippets()?;
    let map = snippets
        .as_object()
        .ok_or_else(|| "snippets root is not a JSON object".to_string())?;
    for (name, snippet) in map {
        assert!(
            !snippet["prefix"].is_null(),
            "snippet '{name}' is missing 'prefix' field"
        );
        assert!(
            !snippet["body"].is_null(),
            "snippet '{name}' is missing 'body' field"
        );
        assert!(
            !snippet["description"].is_null(),
            "snippet '{name}' is missing 'description' field"
        );
    }
    Ok(())
}

#[test]
fn test_snippet_body_non_empty() -> Result<(), String> {
    let snippets = load_snippets()?;
    let map = snippets
        .as_object()
        .ok_or_else(|| "snippets root is not a JSON object".to_string())?;
    for (name, snippet) in map {
        match &snippet["body"] {
            serde_json::Value::Array(lines) => {
                assert!(
                    !lines.is_empty(),
                    "snippet '{name}' has an empty body array"
                );
            }
            serde_json::Value::String(s) => {
                assert!(!s.is_empty(), "snippet '{name}' has an empty body string");
            }
            other => {
                return Err(format!(
                    "snippet '{name}' has unexpected body type: {other:?}"
                ));
            }
        }
    }
    Ok(())
}
