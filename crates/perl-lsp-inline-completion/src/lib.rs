//! Inline completions provider with deterministic rules
//!
//! This crate provides context-aware inline completions that appear as
//! ghost text. These are deterministic completions based on patterns,
//! not AI-powered suggestions.

use perl_position_tracking::utf16_line_col_to_offset;
use serde::{Deserialize, Serialize};

/// Prepared context for inline completion suggestions and future AI handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedInlineCompletionContext {
    /// Prefix on the current line up to the request position.
    pub prefix: String,
    /// Full current line with trailing newline removed.
    pub current_line: String,
    /// Closest previous non-empty line, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_non_empty_line: Option<String>,
    /// Nearest enclosing subroutine name, if one can be inferred.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_function: Option<String>,
    /// Nearest package declaration before the cursor, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_package: Option<String>,
    /// Nearby variables, ordered from closest to farthest.
    pub variables: Vec<String>,
    /// Imported modules or pragmas visible before the cursor.
    pub imports: Vec<String>,
}

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
        if let Some(context) = self.prepare_context(text, line, character) {
            let items = self.get_completions_for_context(&context);
            return InlineCompletionList { items };
        }

        InlineCompletionList { items: vec![] }
    }

    /// Prepare surrounding code context for deterministic suggestions and
    /// future LLM-backed inline completion.
    pub fn prepare_context(
        &self,
        text: &str,
        line: u32,
        character: u32,
    ) -> Option<PreparedInlineCompletionContext> {
        let line_context = self.line_context_at_position(text, line, character)?;
        let lines = self.normalized_lines(text);
        let line_index = usize::try_from(line).ok()?;
        let (current_function, function_start_line) =
            self.current_function_context(&lines, line_index);
        let visible_text = self.visible_text_until_cursor(&lines, line_index, line_context.prefix);
        let variable_scan_text = self.visible_text_since_line(
            &lines,
            function_start_line.unwrap_or(0),
            line_index,
            line_context.prefix,
        );

        Some(PreparedInlineCompletionContext {
            prefix: line_context.prefix.to_string(),
            current_line: line_context.current_line.to_string(),
            previous_non_empty_line: self
                .previous_non_empty_line(&lines, line_index)
                .map(str::to_string),
            current_function,
            current_package: self.current_package(&lines, line_index),
            variables: self.collect_variables(&variable_scan_text),
            imports: self.collect_imports(&visible_text),
        })
    }

    fn line_context_at_position<'a>(
        &self,
        text: &'a str,
        line: u32,
        character: u32,
    ) -> Option<LineContext<'a>> {
        let lines = self.normalized_lines(text);
        let line_index = usize::try_from(line).ok()?;
        let current_line = *lines.get(line_index)?;
        let prefix_end = utf16_line_col_to_offset(current_line, 0, character);

        Some(LineContext { prefix: &current_line[..prefix_end], current_line })
    }

    fn normalized_lines<'a>(&self, text: &'a str) -> Vec<&'a str> {
        if text.is_empty() {
            return vec![""];
        }

        text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line)).collect()
    }

    fn get_completions_for_context(
        &self,
        context: &PreparedInlineCompletionContext,
    ) -> Vec<InlineCompletionItem> {
        let prefix = context.prefix.as_str();
        let full_line = context.current_line.as_str();
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
        if prefix.ends_with("return ") {
            if let Some(variable) = self.preferred_return_variable(context) {
                items.push(InlineCompletionItem {
                    insert_text: format!("{variable};"),
                    filter_text: Some(variable),
                    range: None,
                    command: None,
                });
            } else if self.is_in_constructor_context(context.current_function.as_deref(), prefix) {
                items.push(InlineCompletionItem {
                    insert_text: "$self;".into(),
                    filter_text: Some("$self".into()),
                    range: None,
                    command: None,
                });
            }
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

        self.add_contextual_fallbacks(context, &mut items);
        self.deduplicate_items(&mut items);
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
    fn is_in_constructor_context(&self, current_function: Option<&str>, prefix: &str) -> bool {
        matches!(current_function, Some("new" | "BUILD"))
            || prefix.contains("sub new")
            || prefix.contains("sub BUILD")
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

    fn current_function_context(
        &self,
        lines: &[&str],
        line_index: usize,
    ) -> (Option<String>, Option<usize>) {
        lines.iter().take(line_index + 1).enumerate().fold(
            (None, None),
            |mut state, (idx, line)| {
                if let Some(name) = self.parse_sub_name(line) {
                    state = (Some(name), Some(idx));
                }
                state
            },
        )
    }

    fn current_package(&self, lines: &[&str], line_index: usize) -> Option<String> {
        lines.iter().take(line_index + 1).filter_map(|line| self.parse_package_name(line)).last()
    }

    fn previous_non_empty_line<'a>(
        &self,
        lines: &'a [&'a str],
        line_index: usize,
    ) -> Option<&'a str> {
        lines
            .get(..line_index)
            .and_then(|slice| slice.iter().rev().find(|line| !line.trim().is_empty()).copied())
    }

    fn visible_text_until_cursor(&self, lines: &[&str], line_index: usize, prefix: &str) -> String {
        self.visible_text_since_line(lines, 0, line_index, prefix)
    }

    fn visible_text_since_line(
        &self,
        lines: &[&str],
        start_line: usize,
        line_index: usize,
        prefix: &str,
    ) -> String {
        let mut visible_text = String::new();

        for (idx, line) in
            lines.iter().enumerate().skip(start_line).take(line_index.saturating_sub(start_line))
        {
            if idx > start_line {
                visible_text.push('\n');
            }
            visible_text.push_str(line);
        }

        if line_index > start_line || !visible_text.is_empty() {
            visible_text.push('\n');
        }
        visible_text.push_str(prefix);
        visible_text
    }

    fn collect_imports(&self, visible_text: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in visible_text.lines() {
            if let Some(import_name) = self.parse_use_name(line) {
                self.push_unique(&mut imports, import_name);
            }
        }

        imports
    }

    fn collect_variables(&self, visible_text: &str) -> Vec<String> {
        let mut matches = Vec::new();
        let bytes = visible_text.as_bytes();
        let mut index = 0usize;

        while index < bytes.len() {
            let byte = bytes[index];
            if byte == b'$' || byte == b'@' || byte == b'%' {
                let start = index;
                index += 1;

                if index >= bytes.len() {
                    break;
                }

                let first = bytes[index] as char;
                if !(first.is_ascii_alphabetic() || first == '_') {
                    continue;
                }

                index += 1;
                while index < bytes.len() {
                    let next = bytes[index] as char;
                    if next.is_ascii_alphanumeric() || next == '_' {
                        index += 1;
                    } else {
                        break;
                    }
                }

                matches.push(visible_text[start..index].to_string());
                continue;
            }

            index += 1;
        }

        let mut variables = Vec::new();
        for variable in matches.into_iter().rev() {
            self.push_unique(&mut variables, variable);
            if variables.len() >= 8 {
                break;
            }
        }

        variables
    }

    fn parse_use_name(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("use ")?;
        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_'))
            .collect();

        (!name.is_empty()).then_some(name)
    }

    fn parse_sub_name(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("sub ")?;
        let name: String = rest
            .chars()
            .skip_while(|ch| ch.is_whitespace())
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect();

        (!name.is_empty()).then_some(name)
    }

    fn parse_package_name(&self, line: &str) -> Option<String> {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("package ")?;
        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_'))
            .collect();

        (!name.is_empty()).then_some(name)
    }

    fn add_contextual_fallbacks(
        &self,
        context: &PreparedInlineCompletionContext,
        items: &mut Vec<InlineCompletionItem>,
    ) {
        let prefix = context.prefix.trim();
        let comment_context = context
            .previous_non_empty_line
            .as_deref()
            .map(|line| line.trim_start().starts_with('#'))
            .unwrap_or(false);

        if context.current_line.is_empty()
            && context.current_function.is_none()
            && context.imports.is_empty()
            && context.variables.is_empty()
        {
            items.push(InlineCompletionItem {
                insert_text: "#!/usr/bin/env perl\nuse strict;\nuse warnings;\n\n".into(),
                filter_text: Some("perl".into()),
                range: None,
                command: None,
            });
            items.push(InlineCompletionItem {
                insert_text: "use strict;\nuse warnings;\n\n".into(),
                filter_text: Some("strict".into()),
                range: None,
                command: None,
            });
        }

        if prefix.is_empty() {
            if let Some(variable) = self.preferred_return_variable(context) {
                items.push(InlineCompletionItem {
                    insert_text: format!("return {variable};"),
                    filter_text: Some(variable),
                    range: None,
                    command: None,
                });
            }

            if self.imports_include(context, "Test::More")
                || self.imports_include(context, "Test2::V0")
            {
                items.push(InlineCompletionItem {
                    insert_text: "done_testing();".into(),
                    filter_text: Some("done_testing".into()),
                    range: None,
                    command: None,
                });
            }

            if comment_context && let Some(variable) = self.preferred_assignment_variable(context) {
                items.push(InlineCompletionItem {
                    insert_text: format!("my {variable} = shift;"),
                    filter_text: Some(variable),
                    range: None,
                    command: None,
                });
            }
        }
    }

    fn preferred_return_variable(
        &self,
        context: &PreparedInlineCompletionContext,
    ) -> Option<String> {
        context
            .variables
            .iter()
            .find(|variable| variable.as_str() == "$self")
            .cloned()
            .or_else(|| context.variables.first().cloned())
    }

    fn preferred_assignment_variable(
        &self,
        context: &PreparedInlineCompletionContext,
    ) -> Option<String> {
        context
            .variables
            .iter()
            .find(|variable| variable.starts_with('$') && variable.as_str() != "$self")
            .cloned()
    }

    fn imports_include(&self, context: &PreparedInlineCompletionContext, expected: &str) -> bool {
        context.imports.iter().any(|import_name| import_name == expected)
    }

    fn deduplicate_items(&self, items: &mut Vec<InlineCompletionItem>) {
        let mut seen = Vec::<String>::new();
        items.retain(|item| {
            if seen.iter().any(|existing| existing == &item.insert_text) {
                return false;
            }
            seen.push(item.insert_text.clone());
            true
        });
    }

    fn push_unique(&self, values: &mut Vec<String>, value: String) {
        if values.iter().any(|existing| existing == &value) {
            return;
        }
        values.push(value);
    }
}

struct LineContext<'a> {
    prefix: &'a str,
    current_line: &'a str,
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

    #[test]
    fn test_prepare_context_collects_function_variables_and_imports() {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\npackage Demo;\n\nsub helper {\n    my $result = 1;\n    my $status = $result;\n    \n}\n";
        let line = 6;
        let character = 4;
        let context =
            provider.prepare_context(source, line, character).expect("expected prepared context");

        assert_eq!(context.current_function.as_deref(), Some("helper"));
        assert_eq!(context.current_package.as_deref(), Some("Demo"));
        assert_eq!(context.previous_non_empty_line.as_deref(), Some("    my $status = $result;"));
        assert!(context.imports.iter().any(|import_name| import_name == "Test::More"));
        assert!(context.variables.iter().any(|variable| variable == "$status"));
        assert!(context.variables.iter().any(|variable| variable == "$result"));
    }

    #[test]
    fn test_empty_file_gets_scaffold_suggestions() {
        let provider = InlineCompletionProvider::new();
        let completions = provider.get_inline_completions("", 0, 0);

        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|item| item.insert_text.contains("use strict;")));
    }

    #[test]
    fn test_blank_line_in_function_prefers_nearby_variable() {
        let provider = InlineCompletionProvider::new();
        let source = "sub helper {\n    my $result = compute();\n    \n}\n";
        let completions = provider.get_inline_completions(source, 2, 4);

        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|item| item.insert_text == "return $result;"));
    }

    #[test]
    fn test_blank_line_after_comment_still_has_contextual_suggestions() {
        let provider = InlineCompletionProvider::new();
        let source = "use Test::More;\n\nsub helper {\n    my $result = 1;\n    # explain next step\n    \n}\n";
        let completions = provider.get_inline_completions(source, 5, 4);

        assert!(!completions.items.is_empty());
        assert!(completions.items.iter().any(|item| item.insert_text == "return $result;"));
        assert!(completions.items.iter().any(|item| item.insert_text == "done_testing();"));
    }
}
