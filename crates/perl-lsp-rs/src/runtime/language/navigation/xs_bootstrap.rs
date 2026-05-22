//! XS bootstrap target extraction and location helpers.
//!
//! This module keeps XS-specific navigation concerns out of the main LSP
//! navigation handler: parsing supported bootstrap call shapes, normalizing
//! module names, and mapping modules to their generated `boot_*` symbols.

use crate::runtime::location_from_path;
use crate::state::normalize_package_separator;
use crate::util::{byte_to_line_col, read_text_file_with_encoding};
use serde_json::{Value, json};
use std::path::Path;
use url::Url;

fn is_module_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'\'')
}

fn normalize_bootstrap_module(token: &str, current_package: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }

    if trimmed == "__PACKAGE__" {
        return Some(current_package.to_string());
    }

    let normalized = normalize_package_separator(trimmed).into_owned();
    let first = normalized.chars().next()?;
    if normalized.contains("::") || first.is_ascii_uppercase() { Some(normalized) } else { None }
}

fn parse_bootstrap_argument(
    text: &str,
    mut start: usize,
    current_package: &str,
) -> Option<(usize, usize, String)> {
    while let Some(byte) = text.as_bytes().get(start) {
        if byte.is_ascii_whitespace() || *byte == b',' {
            start += 1;
        } else {
            break;
        }
    }

    let bytes = text.as_bytes();
    let byte = *bytes.get(start)?;

    if byte == b'\'' || byte == b'"' {
        let quote = byte;
        let token_start = start + 1;
        let mut end = token_start;
        while let Some(next) = bytes.get(end) {
            if *next == quote {
                break;
            }
            end += 1;
        }
        let token = text.get(token_start..end)?;
        let module = normalize_bootstrap_module(token, current_package)?;
        return Some((token_start, end, module));
    }

    let mut end = start;
    while let Some(next) = bytes.get(end) {
        if is_module_token_byte(*next) {
            end += 1;
        } else {
            break;
        }
    }

    if end <= start {
        return None;
    }

    let token = text.get(start..end)?;
    let module = normalize_bootstrap_module(token, current_package)?;
    Some((start, end, module))
}

fn extract_xs_loader_target(
    text: &str,
    cursor: usize,
    current_package: &str,
    marker: &str,
) -> Option<String> {
    let mut search_from = 0;
    while let Some(found) = text.get(search_from..)?.find(marker) {
        let marker_start = search_from + found;
        let marker_end = marker_start + marker.len();
        let mut arg_start = marker_end;

        while let Some(byte) = text.as_bytes().get(arg_start) {
            if byte.is_ascii_whitespace() {
                arg_start += 1;
            } else {
                break;
            }
        }

        if text.as_bytes().get(arg_start) == Some(&b'(') {
            arg_start += 1;
        }

        if let Some((token_start, token_end, module_name)) =
            parse_bootstrap_argument(text, arg_start, current_package)
            && ((cursor >= marker_start && cursor <= marker_end)
                || (cursor >= token_start && cursor <= token_end))
        {
            return Some(module_name);
        }

        search_from = marker_end;
    }

    None
}

fn extract_bare_bootstrap_target(
    text: &str,
    cursor: usize,
    current_package: &str,
) -> Option<String> {
    let mut search_from = 0;
    let needle = "bootstrap";
    while let Some(found) = text.get(search_from..)?.find(needle) {
        let start = search_from + found;
        let end = start + needle.len();

        let left_ok = start == 0 || !is_module_token_byte(text.as_bytes()[start - 1]);
        let right_ok = end == text.len() || !is_module_token_byte(text.as_bytes()[end]);
        let qualified = start >= 2 && &text[start - 2..start] == "::";
        if !left_ok || !right_ok || qualified {
            search_from = end;
            continue;
        }

        if let Some((token_start, token_end, module_name)) =
            parse_bootstrap_argument(text, end, current_package)
            && ((cursor >= start && cursor <= end)
                || (cursor >= token_start && cursor <= token_end))
        {
            return Some(module_name);
        }

        search_from = end;
    }

    None
}

fn extract_qualified_bootstrap_target(text: &str, cursor: usize) -> Option<String> {
    let mut search_from = 0;
    let needle = "::bootstrap";
    while let Some(found) = text.get(search_from..)?.find(needle) {
        let suffix_start = search_from + found;
        let mut module_start = suffix_start;
        while module_start > 0 && is_module_token_byte(text.as_bytes()[module_start - 1]) {
            module_start -= 1;
        }

        if module_start == suffix_start {
            search_from = suffix_start + needle.len();
            continue;
        }

        let module = text.get(module_start..suffix_start)?;
        let module_name = normalize_bootstrap_module(module, "main")?;
        let full_end = suffix_start + needle.len();
        if cursor >= module_start && cursor <= full_end {
            return Some(module_name);
        }

        search_from = full_end;
    }

    None
}

pub(super) fn extract_xs_bootstrap_target(
    text: &str,
    cursor: usize,
    current_package: &str,
) -> Option<String> {
    extract_xs_loader_target(text, cursor, current_package, "XSLoader::load")
        .or_else(|| {
            extract_xs_loader_target(text, cursor, current_package, "DynaLoader::bootstrap")
        })
        .or_else(|| extract_bare_bootstrap_target(text, cursor, current_package))
        .or_else(|| extract_qualified_bootstrap_target(text, cursor))
}

fn xs_boot_symbol_name(module_name: &str) -> String {
    format!("boot_{}", normalize_package_separator(module_name).replace("::", "__"))
}

pub(super) fn xs_bootstrap_location(path: &Path, module_name: &str) -> Value {
    let uri = Url::from_file_path(path).map(|url| url.to_string()).unwrap_or_default();
    let boot_symbol = xs_boot_symbol_name(module_name);

    if let Ok(text) = read_text_file_with_encoding(path)
        && let Some(offset) = text.find(&boot_symbol)
    {
        let (start_line, start_char) = byte_to_line_col(&text, offset);
        let (end_line, end_char) = byte_to_line_col(&text, offset + boot_symbol.len());
        return json!({
            "uri": uri,
            "range": {
                "start": {"line": start_line, "character": start_char},
                "end": {"line": end_line, "character": end_char},
            },
        });
    }

    location_from_path(path)
}
