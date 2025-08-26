//! Inline completions provider with deterministic rules
//!
//! This module provides context-aware inline completions that appear as
//! ghost text. These are deterministic completions based on patterns,
//! not AI-powered suggestions.

use serde::{Deserialize, Serialize};

/// Inline completion item (LSP 3.18 preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletionItem {
    pub insert_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<lsp_types::Range>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<lsp_types::Command>,
}

/// Inline completion list (LSP 3.18 preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletionList {
    pub items: Vec<InlineCompletionItem>,
}

pub struct InlineCompletionProvider;

impl Default for InlineCompletionProvider {
    fn default() -> Self {
        Self
    }
}

impl InlineCompletionProvider {
    pub fn new() -> Self {
        Self
    }

    /// Get inline completions for the given context
    pub fn get_inline_completions(
        &self,
        text: &str,
        line: u32,
        character: u32,
    ) -> InlineCompletionList {
        let lines: Vec<&str> = text.lines().collect();

        if let Some(current_line) = lines.get(line as usize) {
            let prefix = &current_line[..character.min(current_line.len() as u32) as usize];

            // Get completions based on context
            let items = self.get_completions_for_context(prefix, current_line);

            return InlineCompletionList { items };
        }

        InlineCompletionList { items: vec![] }
    }

    fn get_completions_for_context(
        &self,
        prefix: &str,
        full_line: &str,
    ) -> Vec<InlineCompletionItem> {
        let mut items = Vec::new();

        // Rule 1: After `->` suggest `new()`
        if prefix.ends_with("->") {
            items.push(InlineCompletionItem {
                insert_text: "new()".into(),
                filter_text: Some("new".into()),
                range: None,
                command: None,
            });
        }

        // Rule 2: After `use ` suggest common pragmas
        if prefix.trim_end() == "use" || prefix.ends_with("use ") {
            // Suggest strict first as it's most common
            items.push(InlineCompletionItem {
                insert_text: "strict;".into(),
                filter_text: Some("strict".into()),
                range: None,
                command: None,
            });

            items.push(InlineCompletionItem {
                insert_text: "warnings;".into(),
                filter_text: Some("warnings".into()),
                range: None,
                command: None,
            });

            items.push(InlineCompletionItem {
                insert_text: "feature ':5.36';".into(),
                filter_text: Some("feature".into()),
                range: None,
                command: None,
            });
        }

        // Rule 3: After `sub <name>` without `{`, suggest ` {}`
        if let Some(sub_match) = self.match_sub_declaration(prefix) {
            if !full_line.contains('{') {
                items.push(InlineCompletionItem {
                    insert_text: format!(" {{\n    # TODO: implement {}\n}}", sub_match),
                    filter_text: Some("{".into()),
                    range: None,
                    command: None,
                });
            }
        }

        // Rule 4: After `my $` suggest common variable patterns
        if prefix.ends_with("my $") {
            items.push(InlineCompletionItem {
                insert_text: "self = shift;".into(),
                filter_text: Some("self".into()),
                range: None,
                command: None,
            });
        }

        // Rule 5: After `package ` suggest common suffix patterns
        if prefix.ends_with("package ") {
            items.push(InlineCompletionItem {
                insert_text: "MyPackage;\n\nuse strict;\nuse warnings;".into(),
                filter_text: Some("MyPackage".into()),
                range: None,
                command: None,
            });
        }

        // Rule 6: After `bless ` suggest common patterns
        if prefix.ends_with("bless ") {
            items.push(InlineCompletionItem {
                insert_text: "$self, $class;".into(),
                filter_text: Some("$self".into()),
                range: None,
                command: None,
            });
        }

        // Rule 7: After `return ` in constructor context
        if prefix.ends_with("return ") && self.is_in_constructor_context(prefix) {
            items.push(InlineCompletionItem {
                insert_text: "$self;".into(),
                filter_text: Some("$self".into()),
                range: None,
                command: None,
            });
        }

        // Rule 8: Complete common loops
        if prefix.ends_with("for ") {
            items.push(InlineCompletionItem {
                insert_text: "my $item (@items) {\n    \n}".into(),
                filter_text: Some("my".into()),
                range: None,
                command: None,
            });
        }

        if prefix.ends_with("foreach ") {
            items.push(InlineCompletionItem {
                insert_text: "my $item (@items) {\n    \n}".into(),
                filter_text: Some("my".into()),
                range: None,
                command: None,
            });
        }

        // Rule 9: Complete common test patterns
        if prefix.ends_with("ok(") {
            items.push(InlineCompletionItem {
                insert_text: "$result, 'test description');".into(),
                filter_text: Some("$result".into()),
                range: None,
                command: None,
            });
        }

        if prefix.ends_with("is(") {
            items.push(InlineCompletionItem {
                insert_text: "$got, $expected, 'test description');".into(),
                filter_text: Some("$got".into()),
                range: None,
                command: None,
            });
        }

        // Rule 10: Complete shebang
        if prefix == "#!" || prefix == "#!/" {
            items.push(InlineCompletionItem {
                insert_text: "/usr/bin/env perl".into(),
                filter_text: Some("perl".into()),
                range: None,
                command: None,
            });
        }

        items
    }

    /// Check if we're after a sub declaration without body
    fn match_sub_declaration(&self, prefix: &str) -> Option<String> {
        // Match "sub name" pattern
        if let Some(idx) = prefix.rfind("sub ") {
            let after_sub = &prefix[idx + 4..];
            // Check if we have a name and no opening brace
            if !after_sub.is_empty() && !after_sub.contains('{') && !after_sub.contains('(') {
                // Extract just the sub name
                let name = after_sub.trim();
                if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return Some(name.to_string());
                }
            }
        }
        None
    }

    /// Check if we're in a constructor context (sub new or BUILD)
    fn is_in_constructor_context(&self, prefix: &str) -> bool {
        prefix.contains("sub new") || prefix.contains("sub BUILD")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_after_arrow() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("$obj->", 0, 6);
        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "new()");
    }

    #[test]
    fn test_after_use() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("use ", 0, 4);
        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|i| i.insert_text == "strict;"));
    }

    #[test]
    fn test_after_sub() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub hello", 0, 9);
        assert!(!completions.items.is_empty());
        assert!(completions.items[0].insert_text.contains("TODO: implement hello"));
    }

    #[test]
    fn test_no_completion_when_brace_exists() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub hello {", 0, 9);
        // Should not suggest brace when one exists
        assert!(completions.items.is_empty() || !completions.items[0].insert_text.contains('{'));
    }

    #[test]
    fn test_shebang_completion() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("#!/", 0, 3);
        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "/usr/bin/env perl");
    }
}
