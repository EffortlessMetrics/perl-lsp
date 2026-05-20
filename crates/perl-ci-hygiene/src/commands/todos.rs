use color_eyre::eyre::{Context, Result};
use regex::Regex;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone)]
struct TodoHit {
    line_text: String,
}

pub(crate) fn check_todos(repo_root: &Path, list_mode: bool) -> Result<i32> {
    let baseline_path = repo_root.join("ci").join("todo_baseline.txt");
    // "xtask" is excluded because it is build tooling whose source code documents and implements
    // the TODO-scanner itself (unwired_scan.rs), producing unavoidable self-referential matches.
    // ".claude" is excluded because it contains ephemeral agent worktrees and tooling state that
    // are gitignored — the scanner should not see them as project source.
    let exclude_dirs = ["target", ".git", ".receipts", ".runs", "archive", "xtask", ".claude"];
    let exclude_files = [
        repo_root.join("ci").join("check_todos.sh"),
        repo_root.join("crates").join("perl-parser").join("tests").join("missing_docs_ac_tests.rs"),
        repo_root
            .join("crates")
            .join("perl-tdd-support")
            .join("src")
            .join("tdd")
            .join("test_generator.rs"),
        repo_root.join("crates").join("perl-ci-hygiene").join("src").join("main.rs"),
        repo_root
            .join("crates")
            .join("perl-ci-hygiene")
            .join("src")
            .join("commands")
            .join("todos.rs"),
        // Perl code-as-string: contains `TODO` as a Perl package bareword, not an unlinked TODO comment.
        repo_root
            .join("crates")
            .join("perl-parser-core")
            .join("tests")
            .join("complex_paren_args_tests.rs"),
    ];

    let todo_re = Regex::new(r"(?i)\b(?:todo|fixme)\b")?;
    let entries = collect_todo_hits(repo_root, &exclude_dirs, &exclude_files, &todo_re)?;

    if list_mode {
        for hit in entries {
            println!("{}", hit.line_text);
        }
        return Ok(0);
    }

    let current_count = entries.len();
    let baseline_count: usize = if baseline_path.is_file() {
        fs::read_to_string(&baseline_path)?
            .trim()
            .parse::<usize>()
            .wrap_err("parsing ci/todo_baseline.txt")?
    } else {
        fs::create_dir_all(baseline_path.parent().unwrap_or(repo_root))?;
        fs::write(&baseline_path, format!("{current_count}\n"))?;
        println!("📝 Creating initial TODO baseline...");
        println!("✅ Baseline established: {current_count}");
        current_count
    };

    println!("🔎 TODO Compliance Audit");
    println!("=======================");
    println!("Current unlinked TODOs: {current_count}");
    println!("Baseline allowed:       {baseline_count}");
    println!();

    if current_count > baseline_count {
        println!(
            "❌ ERROR: Unlinked TODO count increased from {baseline_count} to {current_count}"
        );
        println!(
            "Please link new TODOs to a GitHub issue using the format: TODO(#123): explanation"
        );
        println!();
        println!("New/Unlinked violations:");
        for hit in entries {
            println!("{}", hit.line_text);
        }
        Ok(1)
    } else if current_count < baseline_count {
        println!(
            "🎉 Great job! You reduced the number of unlinked TODOs ({current_count} < {baseline_count})."
        );
        println!(
            "Please update ci/todo_baseline.txt to {current_count} to lock in this improvement."
        );
        println!();
        Ok(0)
    } else {
        println!("✅ TODO count is within baseline limits.");
        Ok(0)
    }
}

fn collect_todo_hits(
    root: &Path,
    exclude_dirs: &[&str],
    exclude_files: &[PathBuf],
    todo_re: &Regex,
) -> Result<Vec<TodoHit>> {
    let hash_ext = ["sh", "bash", "pl", "pm", "t", "just"];

    let mut hits = Vec::new();

    for entry in WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .with_context(|| format!("path under {:?}", root))?
            .to_path_buf();
        if exclude_files.iter().any(|p| p == path) {
            continue;
        }
        if rel.components().any(|component| {
            exclude_dirs.iter().any(|name| component.as_os_str() == OsStr::new(name))
        }) {
            continue;
        }
        let is_rust = path.extension().is_some_and(|ext| ext == "rs");
        let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        let is_hash_file = file_name == "Justfile"
            || file_name == "justfile"
            || hash_ext.iter().any(|ext| path.extension().is_some_and(|e| e == *ext));
        if !is_rust && !is_hash_file {
            continue;
        }
        let contents = read_lines(path)?;
        let mut raw_string_state = None;
        let mut in_block_comment = false;
        for (line_no, line) in contents.iter().enumerate() {
            let match_line = if is_rust {
                has_unlinked_todo_in_rust_line_with_context(
                    line,
                    todo_re,
                    &mut raw_string_state,
                    &mut in_block_comment,
                )
            } else if path
                .extension()
                .is_some_and(|ext| matches!(ext.to_str(), Some("pl" | "pm" | "t")))
            {
                has_unlinked_todo_in_perl_line(line, todo_re)
            } else {
                has_unlinked_todo_in_hash_line(line, todo_re)
            };
            if !match_line {
                continue;
            }
            hits.push(TodoHit { line_text: format!("{}:{}:{}", rel.display(), line_no + 1, line) });
        }
    }
    Ok(hits)
}

#[cfg(test)]
pub(crate) fn has_unlinked_todo_in_rust_line(line: &str, token_re: &Regex) -> bool {
    let mut raw_string_state = None;
    let mut in_block_comment = false;
    has_unlinked_todo_in_rust_line_with_context(
        line,
        token_re,
        &mut raw_string_state,
        &mut in_block_comment,
    )
}

#[cfg(test)]
pub(crate) fn has_unlinked_todo_in_rust_line_with_block_context(
    line: &str,
    token_re: &Regex,
    in_block_comment: &mut bool,
) -> bool {
    let mut raw_string_state = None;
    has_unlinked_todo_in_rust_line_with_context(
        line,
        token_re,
        &mut raw_string_state,
        in_block_comment,
    )
}

#[cfg(test)]
pub(crate) fn has_unlinked_todo_in_rust_line_with_state(
    line: &str,
    token_re: &Regex,
    raw_string_state: &mut Option<usize>,
) -> bool {
    let mut in_block_comment = false;
    has_unlinked_todo_in_rust_line_with_context(
        line,
        token_re,
        raw_string_state,
        &mut in_block_comment,
    )
}

fn has_unlinked_todo_in_rust_line_with_context(
    line: &str,
    token_re: &Regex,
    raw_string_state: &mut Option<usize>,
    in_block_comment: &mut bool,
) -> bool {
    if *in_block_comment {
        if let Some(end_idx) = find_block_comment_end(line, 0) {
            let mut found = has_unlinked_token(&line[..end_idx], token_re);
            *in_block_comment = false;
            found |= has_unlinked_todo_in_rust_line_with_context(
                &line[end_idx + 2..],
                token_re,
                raw_string_state,
                in_block_comment,
            );
            return found;
        }
        return has_unlinked_token(line, token_re);
    }

    for (idx, _) in line.match_indices("//") {
        if is_index_in_rust_literal(line, idx, *raw_string_state) {
            continue;
        }
        if is_url_like_hash_comment(line, idx) {
            continue;
        }
        if is_likely_string_literal_comment_start(line, idx) {
            continue;
        }
        return has_unlinked_token(&line[idx + 2..], token_re);
    }

    for (idx, _) in line.match_indices("/*") {
        if is_index_in_rust_literal(line, idx, *raw_string_state) {
            continue;
        }
        if is_likely_string_literal_comment_start(line, idx) {
            continue;
        }
        if let Some(end_idx) = find_block_comment_end(line, idx + 2) {
            if has_unlinked_token(&line[idx + 2..end_idx], token_re) {
                return true;
            }
            continue;
        }
        *in_block_comment = true;
        return has_unlinked_token(&line[idx + 2..], token_re);
    }

    *raw_string_state = rust_raw_string_state_after_line(line, *raw_string_state);
    false
}

fn find_block_comment_end(line: &str, start_idx: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = start_idx;

    while cursor < line.len() {
        let next_open = line[cursor..]
            .match_indices("/*")
            .map(|(rel_idx, _)| cursor + rel_idx)
            .find(|&idx| !is_index_in_rust_literal(line, idx, None));
        let next_close = line[cursor..]
            .match_indices("*/")
            .map(|(rel_idx, _)| cursor + rel_idx)
            .find(|&idx| !is_index_in_rust_literal(line, idx, None));

        match (next_open, next_close) {
            (_, None) => return None,
            (None, Some(close_idx)) => {
                depth -= 1;
                if depth == 0 {
                    return Some(close_idx);
                }
                cursor = close_idx + 2;
            }
            (Some(open_idx), Some(close_idx)) if open_idx < close_idx => {
                depth += 1;
                cursor = open_idx + 2;
            }
            (_, Some(close_idx)) => {
                depth -= 1;
                if depth == 0 {
                    return Some(close_idx);
                }
                cursor = close_idx + 2;
            }
        }
    }

    None
}

pub(crate) fn has_unlinked_todo_in_hash_line(line: &str, token_re: &Regex) -> bool {
    let Some(idx) = find_hash_comment_start(line, false) else {
        return false;
    };
    has_unlinked_token(&line[idx + 1..], token_re)
}

pub(crate) fn has_unlinked_todo_in_perl_line(line: &str, token_re: &Regex) -> bool {
    let Some(idx) = find_hash_comment_start(line, true) else {
        return false;
    };
    has_unlinked_token(&line[idx + 1..], token_re)
}

#[derive(Clone, Copy)]
struct PerlQuoteLikeState {
    close_delimiter: char,
    remaining_closures: u8,
    escaped: bool,
}

fn find_hash_comment_start(line: &str, perl_mode: bool) -> Option<usize> {
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut prev_was_escape_single = false;
    let mut prev_was_escape_double = false;
    let mut prev_was_escape_backtick = false;
    let mut perl_quote_like: Option<PerlQuoteLikeState> = None;

    for (idx, ch) in line.char_indices() {
        if let Some(mut quote_like) = perl_quote_like {
            if quote_like.escaped {
                quote_like.escaped = false;
                perl_quote_like = Some(quote_like);
                continue;
            }
            if ch == '\\' {
                quote_like.escaped = true;
                perl_quote_like = Some(quote_like);
                continue;
            }
            if ch == quote_like.close_delimiter {
                quote_like.remaining_closures = quote_like.remaining_closures.saturating_sub(1);
                if quote_like.remaining_closures == 0 {
                    perl_quote_like = None;
                } else {
                    perl_quote_like = Some(quote_like);
                }
            } else {
                perl_quote_like = Some(quote_like);
            }
            continue;
        }

        if in_single {
            if prev_was_escape_single {
                prev_was_escape_single = false;
                continue;
            }
            if ch == '\\' {
                prev_was_escape_single = true;
                continue;
            }
            if ch == '\'' {
                in_single = false;
                prev_was_escape_single = false;
            }
            continue;
        }
        if in_double {
            if prev_was_escape_double {
                prev_was_escape_double = false;
                continue;
            }
            if ch == '\\' {
                prev_was_escape_double = true;
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            continue;
        }
        if in_backtick {
            if prev_was_escape_backtick {
                prev_was_escape_backtick = false;
                continue;
            }
            if ch == '\\' {
                prev_was_escape_backtick = true;
                continue;
            }
            if ch == '`' {
                in_backtick = false;
            }
            continue;
        }

        if perl_mode && let Some(state) = perl_quote_like_state_at_delimiter(line, idx) {
            perl_quote_like = Some(state);
            continue;
        }

        match ch {
            '\'' => {
                in_single = true;
                prev_was_escape_single = false;
            }
            '"' => {
                in_double = true;
                prev_was_escape_double = false;
            }
            '`' => {
                in_backtick = true;
                prev_was_escape_backtick = false;
            }
            '#' => {
                // Bash/Perl parameter-length expansion (`${#var}`) is not a comment.
                if idx >= 2 && line.as_bytes().get(idx - 2..idx) == Some(b"${") {
                    continue;
                }
                if idx == 0 && line.as_bytes().get(1) == Some(&b'!') {
                    return None;
                }
                if idx == 0 {
                    return Some(idx);
                }
                if perl_mode {
                    return Some(idx);
                }
                if let Some(prev) = line[..idx].chars().next_back()
                    && (prev.is_whitespace()
                        || matches!(
                            prev,
                            ';' | '{' | '}' | '(' | ')' | '[' | ']' | '&' | '|' | '<' | '>' | ','
                        ))
                {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn perl_quote_like_state_at_delimiter(
    line: &str,
    delimiter_idx: usize,
) -> Option<PerlQuoteLikeState> {
    let delimiter = line[delimiter_idx..].chars().next()?;
    if delimiter.is_ascii_alphanumeric() || delimiter.is_ascii_whitespace() || delimiter == '_' {
        return None;
    }

    let prefix = &line[..delimiter_idx];
    let mut op_end = prefix.len();

    while op_end > 0 && prefix.as_bytes()[op_end - 1].is_ascii_whitespace() {
        op_end -= 1;
    }
    if op_end == 0 {
        return None;
    }

    let mut op_start = op_end;
    while op_start > 0 && prefix.as_bytes()[op_start - 1].is_ascii_alphabetic() {
        op_start -= 1;
    }

    if op_start == op_end {
        return None;
    }

    if op_start > 0 {
        let before = prefix.as_bytes()[op_start - 1];
        if before.is_ascii_alphanumeric() || before == b'_' || matches!(before, b'$' | b'@' | b'%')
        {
            return None;
        }
    }

    let op = &prefix[op_start..op_end];
    let remaining_closures = if matches!(op, "s" | "tr" | "y") { 2 } else { 1 };
    let close_delimiter = match delimiter {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        other => other,
    };
    if matches!(op, "m" | "q" | "qq" | "qw" | "qx" | "qr" | "s" | "tr" | "y") {
        Some(PerlQuoteLikeState { close_delimiter, remaining_closures, escaped: false })
    } else {
        None
    }
}

fn is_url_like_hash_comment(line: &str, slash_idx: usize) -> bool {
    if slash_idx == 0 {
        return false;
    }
    let before = line.as_bytes()[slash_idx - 1];
    matches!(before, b'/' | b':' | b'"')
}

fn is_likely_string_literal_comment_start(line: &str, comment_idx: usize) -> bool {
    if comment_idx == 0 {
        return false;
    }
    matches!(line.as_bytes()[comment_idx - 1], b'"' | b'\'' | b'#')
}

fn is_index_in_rust_literal(
    line: &str,
    target_idx: usize,
    initial_raw_hashes: Option<usize>,
) -> bool {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut escape = false;
    let mut raw_hashes = initial_raw_hashes;

    while i < bytes.len() && i < target_idx {
        if let Some(hash_count) = raw_hashes {
            if bytes[i] == b'"'
                && i + 1 + hash_count <= bytes.len()
                && bytes[i + 1..i + 1 + hash_count].iter().all(|&b| b == b'#')
            {
                raw_hashes = None;
                i += 1 + hash_count;
                continue;
            }
            i += 1;
            continue;
        }

        if in_string {
            if escape {
                escape = false;
            } else if bytes[i] == b'\\' {
                escape = true;
            } else if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if in_char {
            if escape {
                escape = false;
            } else if bytes[i] == b'\\' {
                escape = true;
            } else if bytes[i] == b'\'' {
                in_char = false;
            }
            i += 1;
            continue;
        }

        if is_prefixed_string_start(bytes, i, b'b') || is_prefixed_string_start(bytes, i, b'c') {
            in_string = true;
            i += 2;
            continue;
        }

        let raw_prefix_len = raw_string_prefix_len(bytes, i);
        if raw_prefix_len > 0 {
            let mut j = i + raw_prefix_len;
            while j < bytes.len() && bytes[j] == b'#' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                raw_hashes = Some(j.saturating_sub(i + raw_prefix_len));
                i = j + 1;
                continue;
            }
        }

        if bytes[i] == b'"' {
            in_string = true;
            i += 1;
            continue;
        }

        if bytes[i] == b'\'' {
            in_char = true;
            i += 1;
            continue;
        }

        i += 1;
    }

    in_string || in_char || raw_hashes.is_some()
}

fn is_prefixed_string_start(bytes: &[u8], idx: usize, prefix: u8) -> bool {
    bytes[idx] == prefix && idx + 1 < bytes.len() && bytes[idx + 1] == b'"'
}

fn raw_string_prefix_len(bytes: &[u8], idx: usize) -> usize {
    if bytes[idx] == b'r' {
        return 1;
    }
    if idx + 1 < bytes.len() && (bytes[idx] == b'b' || bytes[idx] == b'c') && bytes[idx + 1] == b'r'
    {
        return 2;
    }
    0
}

fn rust_raw_string_state_after_line(
    line: &str,
    initial_raw_hashes: Option<usize>,
) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_string = false;
    let mut in_char = false;
    let mut escape = false;
    let mut raw_hashes = initial_raw_hashes;

    while i < bytes.len() {
        if let Some(hash_count) = raw_hashes {
            if bytes[i] == b'"'
                && i + 1 + hash_count <= bytes.len()
                && bytes[i + 1..i + 1 + hash_count].iter().all(|&b| b == b'#')
            {
                raw_hashes = None;
                i += 1 + hash_count;
                continue;
            }
            i += 1;
            continue;
        }

        if in_string {
            if escape {
                escape = false;
            } else if bytes[i] == b'\\' {
                escape = true;
            } else if bytes[i] == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if in_char {
            if escape {
                escape = false;
            } else if bytes[i] == b'\\' {
                escape = true;
            } else if bytes[i] == b'\'' {
                in_char = false;
            }
            i += 1;
            continue;
        }

        if is_prefixed_string_start(bytes, i, b'b') || is_prefixed_string_start(bytes, i, b'c') {
            in_string = true;
            i += 2;
            continue;
        }

        let raw_prefix_len = raw_string_prefix_len(bytes, i);
        if raw_prefix_len > 0 {
            let mut j = i + raw_prefix_len;
            while j < bytes.len() && bytes[j] == b'#' {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'"' {
                raw_hashes = Some(j.saturating_sub(i + raw_prefix_len));
                i = j + 1;
                continue;
            }
        }

        if bytes[i] == b'"' {
            in_string = true;
            i += 1;
            continue;
        }

        if bytes[i] == b'\'' {
            in_char = true;
            i += 1;
            continue;
        }

        i += 1;
    }

    raw_hashes
}

fn has_unlinked_token(comment: &str, token_re: &Regex) -> bool {
    for m in token_re.find_iter(comment) {
        let suffix = &comment[m.end()..];
        if !linked_marker(suffix) {
            return true;
        }
    }
    let upper_comment = comment.to_ascii_uppercase();
    for token in ["TODO", "FIXME"] {
        for (idx, _) in upper_comment.match_indices(token) {
            if !is_ascii_word_boundary(comment, idx, idx + token.len()) {
                continue;
            }
            let suffix = &comment[idx + token.len()..];
            if !linked_marker(suffix) {
                return true;
            }
        }
    }
    false
}

fn is_ascii_word_boundary(s: &str, start: usize, end: usize) -> bool {
    let bytes = s.as_bytes();
    let prev_ok =
        start == 0 || !bytes[start - 1].is_ascii_alphanumeric() && bytes[start - 1] != b'_';
    let next_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric() && bytes[end] != b'_';
    prev_ok && next_ok
}

pub(crate) fn linked_marker(suffix: &str) -> bool {
    let mut suffix = suffix.trim_start();
    while let Some(next) = suffix.strip_prefix(':').or_else(|| suffix.strip_prefix('-')) {
        suffix = next.trim_start();
    }

    let Some(rest) = suffix.strip_prefix("(#") else {
        return false;
    };
    let mut digits = 0;
    for c in rest.chars() {
        if c.is_ascii_digit() {
            digits += 1;
            continue;
        }
        break;
    }
    if digits == 0 {
        return false;
    }
    rest[digits..].starts_with(")")
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(contents.lines().map(str::to_owned).collect())
}
