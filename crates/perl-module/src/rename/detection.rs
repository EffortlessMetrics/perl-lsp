use super::rewriting::index_is_in_quote_or_comment;

pub(super) fn line_references_moose_moo_dsl(line: &str, module_name: &str) -> bool {
    if line.is_empty() || module_name.is_empty() {
        return false;
    }
    let trimmed = line.trim_start();
    let is_extends =
        trimmed == "extends" || trimmed.starts_with("extends ") || trimmed.starts_with("extends(");
    let is_with = trimmed == "with" || trimmed.starts_with("with ") || trimmed.starts_with("with(");
    if !is_extends && !is_with {
        return false;
    }
    crate::token::contains_module_token(line, module_name)
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
    let trimmed = line.trim_start();
    if trimmed.starts_with("package ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("require ")
        || trimmed.starts_with("no ")
    {
        return false;
    }
    for separator in ["::", "'"] {
        let needle = format!("{module_name}{separator}");
        let needle_bytes = needle.as_bytes();
        let line_bytes = line.as_bytes();
        let needle_len = needle_bytes.len();

        if line_bytes.len() < needle_len {
            continue;
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
