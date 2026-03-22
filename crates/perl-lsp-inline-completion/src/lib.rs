//! Inline completions provider with deterministic rules
//!
//! This crate provides context-aware inline completions that appear as
//! ghost text. These are deterministic completions based on patterns,
//! not AI-powered suggestions.

use perl_position_tracking::utf16_line_col_to_offset;
use serde::{Deserialize, Serialize};

/// Inline completion item (LSP 3.18 preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletionItem {
    /// The text to be inserted.
    pub insert_text: String,
    /// The text to be used for filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_text: Option<String>,
    /// The range to be replaced by the completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<lsp_types::Range>,
    /// An optional command to be executed after the completion is inserted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<lsp_types::Command>,
}

/// Inline completion list (LSP 3.18 preview)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineCompletionList {
    /// The inline completion items.
    pub items: Vec<InlineCompletionItem>,
}

/// A provider for inline completions.
pub struct InlineCompletionProvider;

impl Default for InlineCompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineCompletionProvider {
    /// Creates a new `InlineCompletionProvider`.
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
        if let Some((prefix, current_line)) = self.line_context_at_position(text, line, character) {
            let items = self.get_completions_for_context(prefix, current_line);
            return InlineCompletionList { items };
        }

        InlineCompletionList { items: vec![] }
    }

    fn line_context_at_position<'a>(
        &self,
        text: &'a str,
        line: u32,
        character: u32,
    ) -> Option<(&'a str, &'a str)> {
        if text.is_empty() {
            return (line == 0).then_some(("", ""));
        }

        for (line_idx, raw_line) in text.split_inclusive('\n').enumerate() {
            if line_idx as u32 == line {
                let current_line = raw_line
                    .strip_suffix("\r\n")
                    .or_else(|| raw_line.strip_suffix('\n'))
                    .unwrap_or(raw_line);
                let prefix_end = utf16_line_col_to_offset(current_line, 0, character);
                return Some((&current_line[..prefix_end], current_line));
            }
        }

        None
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

        // Rule 3: After `sub <name>` without `{`, suggest smart body based on name pattern
        if let Some(sub_name) = self.match_sub_declaration(prefix) {
            if !full_line.contains('{') {
                let body = self.generate_smart_body(&sub_name);
                items.push(InlineCompletionItem {
                    insert_text: format!(" {{\n{}\n}}", body),
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

    /// Generate a smart subroutine body based on naming patterns
    ///
    /// Detects common Perl subroutine naming conventions and generates
    /// appropriate body templates:
    /// - `new`, `BUILD` -> constructor pattern
    /// - `get_*` -> getter pattern
    /// - `set_*` -> setter pattern
    /// - `is_*`, `has_*`, `can_*` -> boolean accessor pattern
    /// - `_*` -> private method placeholder
    /// - default -> simple method template
    fn generate_smart_body(&self, sub_name: &str) -> String {
        // Constructor patterns
        if sub_name == "new" || sub_name == "BUILD" {
            return "    my $class = shift;\n    my $self = bless {}, $class;\n    return $self;"
                .to_string();
        }

        // Getter pattern: get_something or something_getter
        if let Some(field) = sub_name.strip_prefix("get_") {
            // Remove "get_" prefix
            return format!("    my $self = shift;\n    return $self->{{{}}};", field);
        }

        // Setter pattern: set_something or something_setter
        if let Some(field) = sub_name.strip_prefix("set_") {
            // Remove "set_" prefix
            return format!(
                "    my ($self, $value) = @_;\n    $self->{{{}}} = $value;\n    return $self;",
                field
            );
        }

        // Boolean accessor patterns: is_*, has_*, can_*
        if sub_name.starts_with("is_")
            || sub_name.starts_with("has_")
            || sub_name.starts_with("can_")
        {
            let prefix_len = if sub_name.starts_with("is_") { 3 } else { 4 };
            let field = &sub_name[prefix_len..];
            return format!("    my $self = shift;\n    return $self->{{{}}} ? 1 : 0;", field);
        }

        // Private method placeholder
        if sub_name.starts_with('_') {
            return "    my $self = shift;\n    ...".to_string();
        }

        // Default: simple method with shift
        "    my $self = shift;\n    ...".to_string()
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
        // Default method generates simple template with shift
        assert!(completions.items[0].insert_text.contains("my $self = shift"));
    }

    #[test]
    fn test_sub_new_constructor() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub new", 0, 7);
        assert!(!completions.items.is_empty());
        // Constructor generates bless pattern
        assert!(completions.items[0].insert_text.contains("bless"));
        assert!(completions.items[0].insert_text.contains("my $class = shift"));
    }

    #[test]
    fn test_sub_getter() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub get_name", 0, 12);
        assert!(!completions.items.is_empty());
        // Getter generates accessor pattern
        assert!(completions.items[0].insert_text.contains("return $self->{name}"));
    }

    #[test]
    fn test_sub_setter() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub set_name", 0, 12);
        assert!(!completions.items.is_empty());
        // Setter generates mutator pattern
        assert!(completions.items[0].insert_text.contains("$self->{name} = $value"));
    }

    #[test]
    fn test_sub_is_predicate() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub is_active", 0, 13);
        assert!(!completions.items.is_empty());
        // Boolean accessor returns 1/0
        assert!(completions.items[0].insert_text.contains("? 1 : 0"));
    }

    #[test]
    fn test_sub_has_predicate() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("sub has_items", 0, 13);
        assert!(!completions.items.is_empty());
        // Boolean accessor returns 1/0
        assert!(completions.items[0].insert_text.contains("? 1 : 0"));
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

    #[test]
    fn test_after_arrow_with_unicode_prefix_uses_utf16_position() {
        let provider = InlineCompletionProvider::new();
        let source = "my $emoji = \"😀\"; my $obj = Package->";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "new()");
    }

    /// 2-byte UTF-8 characters (U+0080..U+07FF, e.g. accented Latin) have 1 UTF-16 code unit
    /// but 2 UTF-8 bytes. The old byte-slice implementation would slice at byte index
    /// equal to the UTF-16 column, which could fall on a continuation byte (0x80-0xBF)
    /// and cause a panic. The microcrate uses utf16_line_col_to_offset and is safe.
    #[test]
    fn test_after_arrow_with_latin1_prefix_uses_utf16_position() {
        let provider = InlineCompletionProvider::new();
        // 'é' is U+00E9 = 2 UTF-8 bytes (0xC3 0xA9) but 1 UTF-16 code unit.
        // UTF-8 byte count of source = 5; UTF-16 unit count = 4.
        // Old byte-slice at col=4 returns "xé-" (missing '>'), so no completion.
        let source = "xé->";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "new()");
    }

    /// CJK characters (U+4E00..U+9FFF, 3 UTF-8 bytes, 1 UTF-16 code unit) present the same
    /// class of offset mismatch as emoji and Latin-1 extended chars.
    #[test]
    fn test_after_arrow_with_cjk_prefix_uses_utf16_position() {
        let provider = InlineCompletionProvider::new();
        // '私' (U+79C1) = 3 UTF-8 bytes (0xE7 0xA7 0x81) but 1 UTF-16 code unit.
        // UTF-8 byte count = 5; UTF-16 unit count = 3.
        // Old byte-slice at col=3 returns "私" (missing "->"), so no completion.
        let source = "私->";
        let character = source.encode_utf16().count() as u32;
        let completions = provider.get_inline_completions(source, 0, character);

        assert!(!completions.items.is_empty());
        assert_eq!(completions.items[0].insert_text, "new()");
    }
}
