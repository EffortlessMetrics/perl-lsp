//! Auto-import logic for completion items.
//!
//! When a completion item refers to a symbol from another module, this module
//! determines whether a `use` statement needs to be added and, if so, computes
//! the additional text edit (insertion point + text) so the LSP client can
//! apply it alongside the completion.

use perl_parser_core::SourceLocation;

#[derive(Debug, Clone)]
struct UseStatement {
    module_name: String,
    line_end: usize,
    imported_symbols: Vec<String>,
    is_pragma: bool,
}

/// Result of computing an auto-import edit.
#[derive(Debug, Clone)]
pub struct AutoImportEdit {
    /// The location where the text should be inserted.
    pub location: SourceLocation,
    /// The text to insert (e.g., `"use Carp qw(croak);\n"`).
    pub text: String,
}

fn parse_use_statements(source: &str) -> Vec<UseStatement> {
    let mut statements = Vec::new();
    let mut byte_offset = 0usize;
    for line in source.split('\n') {
        let line_end = byte_offset + line.len();
        byte_offset = line_end + 1;
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("use ") else {
            continue;
        };
        let rest = rest.trim_start();
        let mod_end = rest
            .find(|c: char| c.is_ascii_whitespace() || c == ';' || c == '(')
            .unwrap_or(rest.len());
        if mod_end == 0 {
            continue;
        }
        let module_name = &rest[..mod_end];
        let is_pragma = module_name.chars().next().is_some_and(|c| c.is_ascii_lowercase());
        let after_module = rest[mod_end..].trim_start();
        let imported_symbols = parse_import_list(after_module);
        let actual_end =
            if source.get(line_end..line_end + 1) == Some("\n") { line_end + 1 } else { line_end };
        statements.push(UseStatement {
            module_name: module_name.to_string(),
            line_end: actual_end,
            imported_symbols,
            is_pragma,
        });
    }
    statements
}

fn parse_import_list(text: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    if let Some(qw_pos) = text.find("qw") {
        let after_qw = text[qw_pos + 2..].trim_start();
        if let Some(first_char) = after_qw.chars().next() {
            let close = match first_char {
                '(' => ')',
                '[' => ']',
                '{' => '}',
                '<' => '>',
                other => other,
            };
            let inside = &after_qw[first_char.len_utf8()..];
            if let Some(end) = inside.find(close) {
                symbols.extend(inside[..end].split_whitespace().map(String::from));
            }
        }
    }
    symbols
}

fn find_insertion_point(source: &str, use_statements: &[UseStatement]) -> usize {
    if let Some(last) = use_statements.iter().rev().find(|u| !u.is_pragma) {
        return last.line_end;
    }
    if let Some(last) = use_statements.last() {
        return last.line_end;
    }
    let mut byte_offset = 0usize;
    for line in source.split('\n') {
        let line_end = byte_offset + line.len();
        if line.trim().starts_with("package ") {
            return if source.get(line_end..line_end + 1) == Some("\n") {
                line_end + 1
            } else {
                line_end
            };
        }
        byte_offset = line_end + 1;
    }
    if source.starts_with("#!") { source.find('\n').map(|p| p + 1).unwrap_or(0) } else { 0 }
}

/// Compute the auto-import edit needed when completing a symbol from another module.
///
/// Returns `None` if the module is already imported or parameters are empty.
pub fn compute_auto_import(
    source: &str,
    module_name: &str,
    symbol_name: &str,
    use_qw_style: bool,
) -> Option<AutoImportEdit> {
    if module_name.is_empty() || symbol_name.is_empty() {
        return None;
    }
    let use_statements = parse_use_statements(source);
    for stmt in &use_statements {
        if stmt.module_name == module_name {
            if !use_qw_style {
                return None;
            }
            if stmt.imported_symbols.contains(&symbol_name.to_string()) {
                return None;
            }
            // Module imported but symbol not in qw list -- don't duplicate the use line
            return None;
        }
    }
    let insertion_point = find_insertion_point(source, &use_statements);
    let import_text = if use_qw_style {
        format!("use {module_name} qw({symbol_name});\n")
    } else {
        format!("use {module_name};\n")
    };
    Some(AutoImportEdit {
        location: SourceLocation { start: insertion_point, end: insertion_point },
        text: import_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_import_basic_qw() {
        let source = "use strict;\nuse warnings;\n\nmy $x = 1;\n";
        let edit = compute_auto_import(source, "Carp", "croak", true);
        assert!(edit.is_some());
        let edit = edit.unwrap();
        assert_eq!(edit.text, "use Carp qw(croak);\n");
        assert!(edit.location.start > 0);
    }

    #[test]
    fn auto_import_without_qw() {
        let source = "use strict;\n\n";
        let edit = compute_auto_import(source, "File::Find", "find", false);
        assert!(edit.is_some());
        assert_eq!(edit.unwrap().text, "use File::Find;\n");
    }

    #[test]
    fn no_import_when_already_present() {
        let source = "use strict;\nuse Carp qw(croak);\n\nmy $x = 1;\n";
        assert!(compute_auto_import(source, "Carp", "croak", true).is_none());
    }

    #[test]
    fn no_import_when_module_already_used() {
        let source = "use strict;\nuse Carp;\n\nmy $x = 1;\n";
        assert!(compute_auto_import(source, "Carp", "croak", false).is_none());
    }

    #[test]
    fn insertion_after_module_use() {
        let source = "use strict;\nuse warnings;\nuse MyApp::Config;\n\nsub foo {}\n";
        let edit = compute_auto_import(source, "Carp", "croak", true).unwrap();
        let expected = source.find("use MyApp::Config;\n").unwrap() + "use MyApp::Config;\n".len();
        assert_eq!(edit.location.start, expected);
    }

    #[test]
    fn insertion_after_package_declaration() {
        let source = "package MyApp;\n\nsub foo {}\n";
        let edit = compute_auto_import(source, "Carp", "croak", true).unwrap();
        assert_eq!(
            edit.location.start,
            source.find("package MyApp;\n").unwrap() + "package MyApp;\n".len()
        );
    }

    #[test]
    fn insertion_at_top_of_file() {
        let edit = compute_auto_import("my $x = 1;\n", "Carp", "croak", true).unwrap();
        assert_eq!(edit.location.start, 0);
    }

    #[test]
    fn insertion_after_shebang() {
        let source = "#!/usr/bin/perl\nmy $x = 1;\n";
        let edit = compute_auto_import(source, "Carp", "croak", true).unwrap();
        assert_eq!(edit.location.start, "#!/usr/bin/perl\n".len());
    }

    #[test]
    fn empty_module_name() {
        assert!(compute_auto_import("use strict;\n", "", "croak", true).is_none());
    }

    #[test]
    fn empty_symbol_name() {
        assert!(compute_auto_import("use strict;\n", "Carp", "", true).is_none());
    }

    #[test]
    fn parse_qw_variants() {
        let source = "use Carp qw(croak confess);\nuse File::Basename qw/basename dirname/;\n";
        let stmts = parse_use_statements(source);
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0].imported_symbols, vec!["croak", "confess"]);
        assert_eq!(stmts[1].imported_symbols, vec!["basename", "dirname"]);
    }

    #[test]
    fn pragma_detection() {
        let source = "use strict;\nuse warnings;\nuse utf8;\nuse MyModule;\n";
        let stmts = parse_use_statements(source);
        assert!(stmts[0].is_pragma);
        assert!(stmts[1].is_pragma);
        assert!(stmts[2].is_pragma);
        assert!(!stmts[3].is_pragma);
    }

    #[test]
    fn module_imported_with_different_symbols() {
        assert!(compute_auto_import("use Carp qw(confess);\n", "Carp", "croak", true).is_none());
    }

    #[test]
    fn auto_import_integration_scenario() {
        let source = "use strict;\nuse warnings;\n\ncro";
        let edit = compute_auto_import(source, "Carp", "croak", true).unwrap();
        assert_eq!(edit.text, "use Carp qw(croak);\n");
        let expected = source.find("use warnings;\n").unwrap() + "use warnings;\n".len();
        assert_eq!(edit.location.start, expected);
    }
}
