//! Conservative native build hint extraction.
//!
//! This module looks for literal `Makefile.PL` / `Build.PL` hints at the
//! workspace root and extracts only native include directories. It does not
//! execute Perl or try to model full build metadata.

use std::fs;
use std::path::Path;

/// Native build hints derived from workspace-root build scripts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeBuildHints {
    /// Native include directories discovered from static build-script hints.
    pub include_dirs: Vec<String>,
}

/// Detect literal native include directories from `Makefile.PL` / `Build.PL`.
pub fn detect_native_build_hints(workspace_root: &Path) -> NativeBuildHints {
    let mut include_dirs = Vec::new();

    let makefile_path = workspace_root.join("Makefile.PL");
    if let Ok(source) = fs::read_to_string(&makefile_path) {
        collect_unique(
            &mut include_dirs,
            extract_makefile_include_dirs(&source).into_iter(),
        );
    }

    let build_pl_path = workspace_root.join("Build.PL");
    if let Ok(source) = fs::read_to_string(&build_pl_path) {
        collect_unique(
            &mut include_dirs,
            extract_build_include_dirs(&source).into_iter(),
        );
    }

    NativeBuildHints { include_dirs }
}

fn extract_makefile_include_dirs(source: &str) -> Vec<String> {
    extract_literal_values_after_key(source, "INC")
        .into_iter()
        .flat_map(|flags| split_include_flags(&flags).into_iter())
        .collect()
}

fn extract_build_include_dirs(source: &str) -> Vec<String> {
    let mut include_dirs = extract_literal_values_after_key(source, "include_dirs");
    let mut extra_flags = extract_literal_values_after_key(source, "extra_compiler_flags")
        .into_iter()
        .flat_map(|flags| split_include_flags(&flags).into_iter())
        .collect::<Vec<_>>();
    include_dirs.append(&mut extra_flags);
    include_dirs
}

fn split_include_flags(flags: &str) -> Vec<String> {
    flags
        .split_whitespace()
        .filter_map(|token| token.strip_prefix("-I"))
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn collect_unique<I>(into: &mut Vec<String>, values: I)
where
    I: Iterator<Item = String>,
{
    for value in values {
        if !value.is_empty() && !into.contains(&value) {
            into.push(value);
        }
    }
}

fn extract_literal_values_after_key(source: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let bytes = source.as_bytes();
    let mut search_from = 0;

    while let Some((_, value_start)) = find_key_assignment(bytes, key, search_from) {
        if let Some((mut parsed, consumed)) = parse_literal_value(source, value_start) {
            values.append(&mut parsed);
            search_from = value_start + consumed;
        } else {
            search_from = value_start + 1;
        }
    }

    values
}

fn find_key_assignment(bytes: &[u8], key: &str, start: usize) -> Option<(usize, usize)> {
    let key_bytes = key.as_bytes();
    let mut idx = start;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_comment = false;

    while idx < bytes.len() {
        let byte = bytes[idx];

        if in_comment {
            idx += 1;
            if byte == b'\n' {
                in_comment = false;
            }
            continue;
        }

        if in_single_quote {
            idx += 1;
            if byte == b'\\' && idx < bytes.len() {
                idx += 1;
                continue;
            }
            if byte == b'\'' {
                in_single_quote = false;
            }
            continue;
        }

        if in_double_quote {
            idx += 1;
            if byte == b'\\' && idx < bytes.len() {
                idx += 1;
                continue;
            }
            if byte == b'"' {
                in_double_quote = false;
            }
            continue;
        }

        match byte {
            b'#' => {
                in_comment = true;
                idx += 1;
                continue;
            }
            b'\'' => {
                in_single_quote = true;
                idx += 1;
                continue;
            }
            b'"' => {
                in_double_quote = true;
                idx += 1;
                continue;
            }
            _ => {}
        }

        if !bytes[idx..].starts_with(key_bytes) {
            idx += 1;
            continue;
        }

        let key_pos = idx;
        if !is_key_boundary(bytes, key_pos, key_bytes.len()) {
            idx = key_pos + key_bytes.len();
            continue;
        }

        let mut value_idx = key_pos + key_bytes.len();
        skip_ws_and_comments(bytes, &mut value_idx);
        if value_idx + 1 >= bytes.len()
            || bytes.get(value_idx) != Some(&b'=')
            || bytes.get(value_idx + 1) != Some(&b'>')
        {
            idx = value_idx.saturating_add(1);
            continue;
        }

        value_idx += 2;
        skip_ws_and_comments(bytes, &mut value_idx);
        return Some((key_pos, value_idx));
    }

    None
}

fn is_key_boundary(bytes: &[u8], key_pos: usize, key_len: usize) -> bool {
    let is_ident = |b: u8| b.is_ascii_alphanumeric() || b == b'_';

    let before_ok = key_pos
        .checked_sub(1)
        .and_then(|idx| bytes.get(idx))
        .is_none_or(|b| !is_ident(*b));
    let after_ok = bytes.get(key_pos + key_len).is_none_or(|b| !is_ident(*b));

    before_ok && after_ok
}

fn skip_ws_and_comments(bytes: &[u8], idx: &mut usize) {
    loop {
        while let Some(b) = bytes.get(*idx) {
            match b {
                b' ' | b'\t' | b'\r' | b'\n' => *idx += 1,
                _ => break,
            }
        }

        if bytes.get(*idx) == Some(&b'#') {
            while *idx < bytes.len() && bytes[*idx] != b'\n' {
                *idx += 1;
            }
            continue;
        }

        break;
    }
}

fn parse_literal_value(source: &str, start: usize) -> Option<(Vec<String>, usize)> {
    let bytes = source.as_bytes();
    match bytes.get(start).copied()? {
        b'\'' | b'"' => {
            let (value, consumed) = parse_quoted_string(source, start)?;
            Some((vec![value], consumed))
        }
        b'[' => parse_quoted_string_array(source, start),
        _ => None,
    }
}

fn parse_quoted_string(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(start)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }

    let mut value = String::new();
    let mut idx = start + 1;
    let mut escaped = false;

    while idx < bytes.len() {
        let ch = source[idx..].chars().next()?;
        let ch_len = ch.len_utf8();
        idx += ch_len;

        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch as u8 == quote {
            return Some((value, idx - start));
        }

        if ch == '\n' {
            return None;
        }

        value.push(ch);
    }

    None
}

fn parse_quoted_string_array(source: &str, start: usize) -> Option<(Vec<String>, usize)> {
    let bytes = source.as_bytes();
    if bytes.get(start) != Some(&b'[') {
        return None;
    }

    let mut idx = start + 1;
    let mut values = Vec::new();

    loop {
        skip_ws_and_comments(bytes, &mut idx);
        match bytes.get(idx).copied()? {
            b']' => return Some((values, idx + 1 - start)),
            b'\'' | b'"' => {
                let (value, consumed) = parse_quoted_string(source, idx)?;
                values.push(value);
                idx += consumed;
                skip_ws_and_comments(bytes, &mut idx);
                match bytes.get(idx).copied()? {
                    b',' => {
                        idx += 1;
                    }
                    b']' => {}
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
}
