//! Deterministic module-import rename edit planning.
//!
//! Computes line edits for Perl module file-rename workflows.

use crate::import_match::line_references_module_import;
use crate::token::{module_variant_pairs, replace_module_token};

/// A full-line replacement edit for a module rename.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLineEdit {
    /// Zero-based source line index.
    pub line: usize,
    /// Start column (always `0` for full-line replacement).
    pub start_character: usize,
    /// End column of the original line in bytes.
    pub end_character: usize,
    /// Replacement text for the full line.
    pub new_text: String,
}

/// Plan full-line edits needed to update module imports after file rename.
///
/// Supported import forms:
/// - `use Module::Name;`
/// - `require Module::Name;`
/// - `use parent 'Module::Name';`
/// - `use parent "Module::Name";`
/// - `use parent qw(Module::Name Other);`
/// - `use base 'Module::Name';`
/// - `use base "Module::Name";`
/// - `use base qw(Module::Name Other);`
///
/// Also rewrites:
/// - Qualified function calls: `Module::Name::func()` → `NewName::func()`
/// - Static method calls: `Module::Name->method()` → `NewName->method()`
/// - `@ISA` array assignments
///
/// Legacy package separators (`Foo'Bar`) are also handled.
#[must_use]
pub fn plan_module_rename_edits(
    source: &str,
    old_module: &str,
    new_module: &str,
) -> Vec<ModuleLineEdit> {
    if source.is_empty()
        || old_module.is_empty()
        || new_module.is_empty()
        || old_module == new_module
    {
        return Vec::new();
    }

    let variants = module_variant_pairs(old_module, new_module);
    let mut edits = Vec::new();

    for (line_idx, line) in source.lines().enumerate() {
        let mut rewritten: Option<String> = None;

        for (old_variant, new_variant) in &variants {
            // Check import forms (use/require/use parent/use base)
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_module_import(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            // Check @ISA assignments
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_isa_assignment(current_line, old_variant) {
                    let (candidate, changed) =
                        replace_module_token(current_line, old_variant, new_variant);
                    if changed {
                        rewritten = Some(candidate);
                    }
                }
            }

            // Check qualified function calls
            {
                let current_line = rewritten.as_deref().unwrap_or(line);
                if line_references_qualified_call(current_line, old_variant) {
                    let candidate =
                        replace_module_name_prefix(current_line, old_variant, new_variant);
                    if candidate != current_line {
                        rewritten = Some(candidate);
                    }
                }
            }
        }

        if let Some(new_text) = rewritten {
            edits.push(ModuleLineEdit {
                line: line_idx,
                start_character: 0,
                end_character: line.len(),
                new_text,
            });
        }
    }

    edits
}

/// Return `true` when `line` contains an `@ISA` assignment that references
/// `module_name` as a standalone token.
#[must_use]
pub fn line_references_isa_assignment(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }
    if !line.contains("@ISA") {
        return false;
    }
    crate::token::contains_module_token(line, module_name)
}

/// Return `true` when `line` contains a qualified call that uses `module_name`
/// as a namespace prefix.
#[must_use]
pub fn line_references_qualified_call(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }
    if line.trim_start().starts_with("package ") {
        return false;
    }
    let needle = format!("{}::", module_name);
    let needle_bytes = needle.as_bytes();
    let line_bytes = line.as_bytes();
    let needle_len = needle_bytes.len();

    if line_bytes.len() < needle_len {
        return false;
    }

    let mut start = 0usize;
    while start + needle_len <= line_bytes.len() {
        let Some(rel) = line[start..].find(needle.as_str()) else {
            break;
        };
        let abs = start + rel;
        let after = abs + needle_len;

        let before_ok = abs == 0 || {
            let ch = line_bytes[abs - 1] as char;
            !ch.is_alphanumeric() && ch != '_' && ch != ':'
        };

        let after_ok = after < line_bytes.len() && {
            let ch = line_bytes[after] as char;
            ch.is_alphabetic() || ch == '_'
        };

        if before_ok && after_ok && !index_is_in_quote_or_comment(line, abs) {
            return true;
        }
        start = abs + 1;
    }

    false
}

/// Return `true` when `line` contains a package declaration referencing
/// `module_name` as a standalone token.
#[must_use]
pub fn line_references_package_declaration(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }
    if !line.trim_start().starts_with("package ") {
        return false;
    }
    crate::token::contains_module_token(line, module_name)
}

/// Replace `old_module::` namespace prefixes in `line` with `new_module::`.
#[must_use]
pub fn replace_module_name_prefix(line: &str, old_module: &str, new_module: &str) -> String {
    if old_module.is_empty() || new_module.is_empty() || line.is_empty() {
        return line.to_string();
    }
    if line.trim_start().starts_with("package ") {
        return line.to_string();
    }

    let needle = format!("{}::", old_module);
    let replacement = format!("{}::", new_module);
    let needle_bytes = needle.as_bytes();
    let needle_len = needle_bytes.len();
    let line_bytes = line.as_bytes();

    if line_bytes.len() < needle_len {
        return line.to_string();
    }

    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;

    while cursor + needle_len <= line_bytes.len() {
        let Some(rel) = line[cursor..].find(needle.as_str()) else {
            break;
        };
        let abs = cursor + rel;
        let after = abs + needle_len;

        let before_ok = abs == 0 || {
            let ch = line_bytes[abs - 1] as char;
            !ch.is_alphanumeric() && ch != '_' && ch != ':'
        };

        let after_ok = after < line_bytes.len() && {
            let ch = line_bytes[after] as char;
            ch.is_alphabetic() || ch == '_'
        };

        if before_ok && after_ok && !index_is_in_quote_or_comment(line, abs) {
            out.push_str(&line[cursor..abs]);
            out.push_str(&replacement);
            cursor = after;
        } else {
            out.push_str(&line[cursor..abs + 1]);
            cursor = abs + 1;
        }
    }

    out.push_str(&line[cursor..]);
    out
}

fn index_is_in_quote_or_comment(line: &str, index: usize) -> bool {
    let bytes = line.as_bytes();
    if index >= bytes.len() {
        return false;
    }

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (i, &byte) in bytes.iter().enumerate() {
        if i == index {
            return in_single || in_double;
        }

        let ch = byte as char;
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            continue;
        }

        if ch == '#' {
            return i < index;
        }

        if ch == '\'' {
            in_single = true;
            continue;
        }

        if ch == '"' {
            in_double = true;
        }
    }

    false
}

/// Apply full-line `ModuleLineEdit` replacements to source text.
#[must_use]
pub fn apply_module_rename_edits(source: &str, edits: &[ModuleLineEdit]) -> String {
    if edits.is_empty() {
        return source.to_string();
    }

    let mut lines: Vec<String> = source.split('\n').map(ToString::to_string).collect();

    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|edit| edit.line);

    for edit in sorted {
        if let Some(line) = lines.get_mut(edit.line) {
            *line = edit.new_text;
        }
    }

    lines.join("\n")
}
