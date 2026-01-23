//! Code Actions for Perl Pragmas
//!
//! Simple quick fixes for adding missing pragmas like use strict and use warnings.

use serde_json::{Value, json};

/// Generate code actions for missing pragmas
pub fn missing_pragmas_actions(uri: &str, text: &str) -> Vec<Value> {
    let has_strict = text.contains("use strict;");
    let has_warnings = text.contains("use warnings;");
    let mut actions = vec![];

    // Find the best insertion point - after package declaration if present,
    // otherwise at the beginning of the file
    let insert_at = find_pragma_insertion_point(text);

    if !has_strict {
        actions.push(make_action(uri, "Add use strict;", insert_at, "use strict;\n"));
    }

    if !has_warnings {
        actions.push(make_action(uri, "Add use warnings;", insert_at, "use warnings;\n"));
    }

    actions
}

/// Find the best position to insert pragmas
fn find_pragma_insertion_point(text: &str) -> usize {
    // Look for package declaration
    if let Some(pos) = text.find("package ") {
        // Find the end of the package line
        if let Some(newline) = text[pos..].find('\n') {
            return pos + newline + 1;
        }
        // If no newline, insert after the package statement
        if let Some(semicolon) = text[pos..].find(';') {
            return pos + semicolon + 1;
        }
    }

    // Otherwise insert at the beginning
    0
}

/// Create a code action JSON object
fn make_action(uri: &str, title: &str, insert_at: usize, snippet: &str) -> Value {
    // Store data for the LSP handler to convert to proper positions
    json!({
        "title": title,
        "kind": "quickfix",
        "data": {
            "uri": uri,
            "insertAt": insert_at,
            "text": snippet
        }
    })
}

/// Generate code actions for auto-fixing perl critic issues
pub fn perl_critic_actions(uri: &str, text: &str, line: u32) -> Vec<Value> {
    let mut actions = vec![];

    // Check if the line has an undefined variable that could use 'my'
    let line_text = text.lines().nth(line as usize).unwrap_or("");

    // Simple pattern: $var without preceding my/our/local
    if line_text.contains('$') || line_text.contains('@') || line_text.contains('%') {
        // Check if it's missing a declaration
        if !line_text.contains("my ")
            && !line_text.contains("our ")
            && !line_text.contains("local ")
        {
            // Find the variable name
            if let Some(var_match) = extract_variable(line_text) {
                actions.push(json!({
                    "title": format!("Add 'my' to {}", var_match),
                    "kind": "quickfix",
                    "data": {
                        "uri": uri,
                        "line": line,
                        "variable": var_match,
                        "declarator": "my"
                    }
                }));
            }
        }
    }

    actions
}

/// Extract a variable name from a line of code
fn extract_variable(line: &str) -> Option<String> {
    // Simple regex-free extraction
    for (i, ch) in line.char_indices() {
        if matches!(ch, '$' | '@' | '%') {
            let rest = &line[i..];
            let end = rest[1..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len() - 1)
                + 1;
            if end > 1 {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_pragmas() {
        let text = r#"
sub hello {
    print "Hello, World!\n";
}
"#;

        let actions = missing_pragmas_actions("file:///test.pl", text);
        assert_eq!(actions.len(), 2);

        let titles: Vec<String> =
            actions.iter().filter_map(|a| a["title"].as_str().map(|s| s.to_string())).collect();

        assert!(titles.contains(&"Add use strict;".to_string()));
        assert!(titles.contains(&"Add use warnings;".to_string()));
    }

    #[test]
    fn test_no_actions_when_pragmas_present() {
        let text = r#"
use strict;
use warnings;

sub hello {
    print "Hello, World!\n";
}
"#;

        let actions = missing_pragmas_actions("file:///test.pl", text);
        assert_eq!(actions.len(), 0);
    }

    #[test]
    fn test_insertion_point_after_package() {
        let text = "package MyModule;\nsub foo {}\n";
        let point = find_pragma_insertion_point(text);
        assert_eq!(point, 18); // After "package MyModule;\n"
    }
}
