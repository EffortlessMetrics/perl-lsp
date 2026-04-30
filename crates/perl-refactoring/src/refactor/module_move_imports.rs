//! Conservative import and qualified-reference rewrites for module moves.
//!
//! This module provides the first mechanical slice of module-move refactoring.
//! It updates obvious `use Old::Name` imports and statically-safe
//! fully-qualified references (`Old::Name::symbol`) while intentionally leaving
//! ambiguous sites (comments, strings, and dynamic constructs) untouched.

use crate::refactor::workspace_refactor::{FileEdit, TextEdit};
use std::path::{Path, PathBuf};

/// Mechanical rewriter for the first module-move import slice.
#[derive(Debug, Clone)]
pub struct ModuleMoveImportRewriter {
    old_module: String,
    new_module: String,
}

impl ModuleMoveImportRewriter {
    /// Create a new module-move import rewriter.
    pub fn new(old_module: impl Into<String>, new_module: impl Into<String>) -> Self {
        Self {
            old_module: old_module.into(),
            new_module: new_module.into(),
        }
    }

    /// Rewrite imports and obvious qualified references for a single file.
    pub fn rewrite_file(&self, file_path: impl AsRef<Path>, source: &str) -> Option<FileEdit> {
        let mut edits = Vec::new();
        let mut offset = 0usize;

        for line in source.split_inclusive('\n') {
            if let Some(new_line) = self.rewrite_line(line) {
                edits.push(TextEdit {
                    start: offset,
                    end: offset + line.len(),
                    new_text: new_line,
                });
            }
            offset += line.len();
        }

        if edits.is_empty() {
            None
        } else {
            Some(FileEdit {
                file_path: file_path.as_ref().to_path_buf(),
                edits,
            })
        }
    }

    /// Rewrite imports and obvious qualified references for many files.
    pub fn rewrite_workspace(&self, files: &[(PathBuf, String)]) -> Vec<FileEdit> {
        files.iter()
            .filter_map(|(path, content)| self.rewrite_file(path, content))
            .collect()
    }

    fn rewrite_line(&self, line: &str) -> Option<String> {
        let (body, newline) = if let Some(stripped) = line.strip_suffix('\n') {
            (stripped, "\n")
        } else {
            (line, "")
        };

        if body.trim_start().starts_with('#') {
            return None;
        }

        if let Some(updated) = self.rewrite_use_statement(body) {
            return Some(format!("{updated}{newline}"));
        }

        if body.trim_start().starts_with("package ") {
            return None;
        }

        self.rewrite_qualified_references(body)
            .map(|updated| format!("{updated}{newline}"))
    }

    fn rewrite_use_statement(&self, line: &str) -> Option<String> {
        let leading_ws = line.len() - line.trim_start().len();
        let mut idx = leading_ws;

        if !line[idx..].starts_with("use") {
            return None;
        }
        idx += 3;

        let mut chars = line[idx..].chars();
        let first = chars.next()?;
        if !first.is_whitespace() {
            return None;
        }

        while let Some(ch) = line[idx..].chars().next() {
            if ch.is_whitespace() {
                idx += ch.len_utf8();
            } else {
                break;
            }
        }

        if !line[idx..].starts_with(&self.old_module) {
            return None;
        }
        let module_end = idx + self.old_module.len();

        if let Some(next) = line[module_end..].chars().next()
            && !(next.is_whitespace() || next == ';')
        {
            return None;
        }

        let mut rewritten = String::with_capacity(line.len() + self.new_module.len());
        rewritten.push_str(&line[..idx]);
        rewritten.push_str(&self.new_module);
        rewritten.push_str(&line[module_end..]);
        Some(rewritten)
    }

    fn rewrite_qualified_references(&self, line: &str) -> Option<String> {
        let code_end = find_unquoted_comment_start(line).unwrap_or(line.len());
        let (code, suffix) = line.split_at(code_end);

        let rewritten_code = rewrite_qualified_in_code(code, &self.old_module, &self.new_module)?;

        Some(format!("{rewritten_code}{suffix}"))
    }
}

fn rewrite_qualified_in_code(code: &str, old_module: &str, new_module: &str) -> Option<String> {
    if old_module.is_empty() {
        return None;
    }

    let mut result = String::with_capacity(code.len());
    let mut idx = 0usize;
    let mut changed = false;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    while idx < code.len() {
        let ch = code[idx..].chars().next()?;
        let ch_len = ch.len_utf8();

        if let Some(active_quote) = quote {
            result.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            idx += ch_len;
            continue;
        }

        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            result.push(ch);
            idx += ch_len;
            continue;
        }

        if code[idx..].starts_with(old_module)
            && code[idx + old_module.len()..].starts_with("::")
            && has_safe_left_boundary(code, idx)
        {
            result.push_str(new_module);
            idx += old_module.len();
            changed = true;
            continue;
        }

        result.push(ch);
        idx += ch_len;
    }

    if changed { Some(result) } else { None }
}

fn has_safe_left_boundary(text: &str, start: usize) -> bool {
    match text[..start].chars().next_back() {
        None => true,
        Some(prev) => !prev.is_alphanumeric() && prev != '_' && prev != ':',
    }
}

fn find_unquoted_comment_start(line: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' => quote = Some(ch),
            '#' => return Some(idx),
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_use_statement() {
        let rewriter = ModuleMoveImportRewriter::new("Old::Name", "New::Name");
        let updated = rewriter.rewrite_line("use Old::Name qw(run);\n");
        assert_eq!(updated.as_deref(), Some("use New::Name qw(run);\n"));
    }

    #[test]
    fn leaves_ambiguous_text_untouched() {
        let rewriter = ModuleMoveImportRewriter::new("Old::Name", "New::Name");
        assert!(rewriter.rewrite_line("my $x = 'Old::Name::run';\n").is_none());
        assert!(rewriter.rewrite_line("# Old::Name::run\n").is_none());
    }
}
