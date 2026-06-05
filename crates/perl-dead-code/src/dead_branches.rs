use crate::{DeadCode, DeadCodeType};
use std::path::Path;

pub(crate) fn detect_dead_branches(file_path: &Path, text: &str, out: &mut Vec<DeadCode>) {
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();
    let mut i = 0;

    while i < n {
        let trimmed = lines[i].trim();

        let dead_reason_and_keyword: Option<(String, &str)> = 'detect: {
            // `for` and `foreach` are list iterators. Falsy list elements such as
            // `0`, `""`, `''`, and `undef` still execute the loop body once.
            for kw in &["if", "while", "elsif", "unless", "until"] {
                let rest = match trimmed.strip_prefix(kw) {
                    Some(r)
                        if r.is_empty()
                            || r.starts_with(|c: char| c.is_whitespace() || c == '(') =>
                    {
                        r.trim_start()
                    }
                    _ => continue,
                };
                if !rest.starts_with('(') {
                    continue;
                }
                let condition = extract_balanced_parens(rest);
                let condition = match condition {
                    Some(c) => c,
                    None => continue,
                };
                // `rest` starts with `(`, condition is `rest[1..idx]`, closing `)` is at
                // index `idx = condition.len() + 1`.  `after_cond` starts at `idx + 1`.
                // We use `.get()` for an explicit bounds-safe slice (#791).
                let after_idx = condition.len() + 2;
                let after_cond = match rest.get(after_idx..) {
                    Some(s) => s.trim(),
                    None => continue,
                };
                if !after_cond.starts_with('{') && !after_cond.is_empty() {
                    continue;
                }
                let inner = condition.trim();

                let reason = if matches!(*kw, "unless" | "until") {
                    if is_always_true(inner) {
                        Some(format!(
                            "`{kw}` condition `{inner}` is always true — block is never executed"
                        ))
                    } else {
                        None
                    }
                } else if is_always_false(inner) {
                    Some(format!(
                        "`{kw}` condition `{inner}` is always false — block is never executed"
                    ))
                } else {
                    None
                };

                if let Some(r) = reason {
                    break 'detect Some((r, *kw));
                }
            }
            None
        };

        if let Some((reason, _kw)) = dead_reason_and_keyword {
            let block_start = i + 1;
            let end_line = find_block_end(&lines, i);
            out.push(DeadCode {
                code_type: DeadCodeType::DeadBranch,
                name: None,
                file_path: file_path.to_path_buf(),
                start_line: block_start,
                end_line,
                reason,
                confidence: 0.9,
                suggestion: Some("Remove this dead branch or fix the condition".to_string()),
            });
            i = end_line;
            continue;
        }

        i += 1;
    }
}

fn is_always_false(condition: &str) -> bool {
    // Strip outer balanced parentheses iteratively to avoid unbounded recursion
    // on adversarially-deep inputs like `((((...0...))))` (#795).
    let c = strip_outer_parens(condition);
    if c == "undef" {
        return true;
    }
    if quoted_literal(c).is_some_and(|inner| inner.is_empty() || inner == "0") {
        return true;
    }
    if c.parse::<i64>().is_ok_and(|n| n == 0) {
        return true;
    }
    if c.parse::<f64>().is_ok_and(|n| n == 0.0) {
        return true;
    }
    false
}

fn is_always_true(condition: &str) -> bool {
    // Strip outer balanced parentheses iteratively to avoid unbounded recursion
    // on adversarially-deep inputs (#795).
    let c = strip_outer_parens(condition);
    if c.parse::<i64>().is_ok_and(|n| n != 0) {
        return true;
    }
    if c.parse::<f64>().is_ok_and(|n| n != 0.0) {
        return true;
    }
    if let Some(inner) = quoted_literal(c) {
        return !inner.is_empty() && inner != "0";
    }
    false
}

/// Strip all layers of balanced outer parentheses from `condition`, returning
/// a reference to the innermost non-paren-wrapped content.
///
/// For example `"(((0)))"` → `"0"`, `"( x )"` → `"x"`, `"0"` → `"0"`.
///
/// This replaces the previous tail-recursive pattern and avoids stack overflow
/// on deeply-nested inputs (#795).
fn strip_outer_parens(condition: &str) -> &str {
    let mut s = condition.trim();
    while s.starts_with('(') && s.ends_with(')') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        // Only strip if the opening '(' matches the closing ')'.
        // E.g. `"(a)(b)"` must NOT be stripped — the first '(' closes at the
        // second character, not at the last ')'.
        if is_outer_paren_balanced(inner) {
            s = inner.trim();
        } else {
            break;
        }
    }
    s
}

/// Returns `true` when wrapping `s` with `(` and `)` would form a balanced
/// pair — i.e., when the first `(` in the parent expression closes at the
/// very last character.  Equivalently, `inner` has a non-negative paren depth
/// at every prefix.
fn is_outer_paren_balanced(inner: &str) -> bool {
    let mut depth = 0i32;
    for ch in inner.chars() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth < 0 {
                    // Closed before end — the `(` wrapping `inner` does NOT
                    // match the trailing `)` we'd strip.
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

fn quoted_literal(condition: &str) -> Option<&str> {
    let is_quoted = condition.starts_with('"') && condition.ends_with('"')
        || condition.starts_with("'") && condition.ends_with("'");
    if is_quoted && condition.len() >= 2 {
        return Some(&condition[1..condition.len() - 1]);
    }
    None
}

fn extract_balanced_parens(s: &str) -> Option<&str> {
    if !s.starts_with('(') {
        return None;
    }
    let mut depth = 0usize;
    for (idx, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[1..idx]);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_block_end(lines: &[&str], open_line: usize) -> usize {
    let mut depth = 0i32;
    for (i, line) in lines.iter().enumerate().skip(open_line) {
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return i + 1;
                    }
                }
                _ => {}
            }
        }
    }
    lines.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_always_false ---

    #[test]
    fn always_false_zero() {
        assert!(is_always_false("0"));
    }

    #[test]
    fn always_false_decimal_zero() {
        assert!(is_always_false("0.0"));
    }

    #[test]
    fn always_false_quoted_zero() {
        assert!(is_always_false("\"0\""));
        assert!(is_always_false("'0'"));
    }

    #[test]
    fn always_false_empty_double_quotes() {
        assert!(is_always_false("\"\""));
    }

    #[test]
    fn always_false_empty_single_quotes() {
        assert!(is_always_false("''"));
    }

    #[test]
    fn always_false_undef() {
        assert!(is_always_false("undef"));
    }

    #[test]
    fn always_false_whitespace_only() {
        // whitespace trims to "" which does not match any literal — not always false
        assert!(!is_always_false("   "));
    }

    #[test]
    fn always_false_empty_string() {
        // empty string does not match any literal
        assert!(!is_always_false(""));
    }

    #[test]
    fn always_false_non_literal_variable() {
        assert!(!is_always_false("$x"));
    }

    #[test]
    fn always_false_non_literal_function_call() {
        assert!(!is_always_false("foo()"));
    }

    #[test]
    fn always_false_one() {
        assert!(!is_always_false("1"));
    }

    #[test]
    fn always_false_nonempty_string_a() {
        assert!(!is_always_false("'a'"));
    }

    #[test]
    fn always_false_nonempty_double_string_x() {
        assert!(!is_always_false("\"x\""));
    }

    #[test]
    fn always_false_wrapped_in_parens() {
        // (0) should be always false due to recursive unwrapping
        assert!(is_always_false("(0)"));
    }

    // --- is_always_true ---

    #[test]
    fn always_true_one() {
        assert!(is_always_true("1"));
    }

    #[test]
    fn always_true_nonzero_int() {
        assert!(is_always_true("42"));
    }

    #[test]
    fn always_true_nonempty_single_string() {
        assert!(is_always_true("'a'"));
    }

    #[test]
    fn always_true_nonempty_double_string() {
        assert!(is_always_true("\"x\""));
    }

    #[test]
    fn always_true_zero_is_not_true() {
        assert!(!is_always_true("0"));
    }

    #[test]
    fn always_true_empty_double_quotes_not_true() {
        assert!(!is_always_true("\"\""));
    }

    #[test]
    fn always_true_empty_single_quotes_not_true() {
        assert!(!is_always_true("''"));
    }

    #[test]
    fn always_true_undef_not_true() {
        assert!(!is_always_true("undef"));
    }

    #[test]
    fn always_true_whitespace_not_true() {
        assert!(!is_always_true("   "));
    }

    #[test]
    fn always_true_empty_not_true() {
        assert!(!is_always_true(""));
    }

    #[test]
    fn always_true_variable_not_true() {
        assert!(!is_always_true("$x"));
    }

    #[test]
    fn always_true_function_call_not_true() {
        assert!(!is_always_true("foo()"));
    }

    #[test]
    fn always_true_string_zero_not_true() {
        // "0" inside quotes — inner is "0" so not always true
        assert!(!is_always_true("\"0\""));
    }

    #[test]
    fn always_true_decimal_zero_not_true() {
        assert!(!is_always_true("0.0"));
    }

    #[test]
    fn always_true_wrapped_in_parens() {
        assert!(is_always_true("(1)"));
    }

    // --- extract_balanced_parens ---

    #[test]
    fn extract_simple() {
        let result = extract_balanced_parens("(a)");
        assert_eq!(result, Some("a"));
    }

    #[test]
    fn extract_nested() {
        let result = extract_balanced_parens("((a)(b))");
        assert_eq!(result, Some("(a)(b)"));
    }

    #[test]
    fn extract_empty_parens() {
        let result = extract_balanced_parens("()");
        assert_eq!(result, Some(""));
    }

    #[test]
    fn extract_unbalanced_returns_none() {
        let result = extract_balanced_parens("(a");
        assert_eq!(result, None);
    }

    #[test]
    fn extract_no_leading_paren_returns_none() {
        let result = extract_balanced_parens("a)");
        assert_eq!(result, None);
    }

    #[test]
    fn extract_with_trailing_content() {
        // only the first balanced group is extracted; trailing content ignored
        let result = extract_balanced_parens("(a) { ... }");
        assert_eq!(result, Some("a"));
    }

    // --- find_block_end ---

    #[test]
    fn find_block_end_simple() {
        let lines = vec!["if (1) {", "    say 'hi';", "}"];
        // open_line=0 means we start scanning from line index 0
        assert_eq!(find_block_end(&lines, 0), 3);
    }

    #[test]
    fn find_block_end_nested() {
        let lines = vec!["if (1) {", "    if (2) {", "    }", "}"];
        assert_eq!(find_block_end(&lines, 0), 4);
    }

    #[test]
    fn find_block_end_missing_close_returns_len() {
        let lines = vec!["if (1) {", "    say 'hi';"];
        // no closing brace — returns lines.len()
        assert_eq!(find_block_end(&lines, 0), 2);
    }

    #[test]
    fn find_block_end_string_braces_counted() {
        // The implementation counts all { and } characters including inside strings.
        // This documents the current behavior: braces in strings are counted naively.
        let lines = vec!["if (1) {", "    my $s = '{nested}';", "}"];
        // line 1: '{nested}' contributes +1 then -1, net 0 extra depth
        // so outer closing brace on line 2 closes at index 2 → returns 3
        assert_eq!(find_block_end(&lines, 0), 3);
    }

    #[test]
    fn find_block_end_starting_mid_slice() {
        let lines = vec!["# preamble", "if (1) {", "    say 'hi';", "}"];
        // start scanning from line 1
        assert_eq!(find_block_end(&lines, 1), 4);
    }

    // --- strip_outer_parens (#795: depth guard) ---

    #[test]
    fn strip_outer_parens_no_parens() {
        assert_eq!(strip_outer_parens("0"), "0");
    }

    #[test]
    fn strip_outer_parens_one_level() {
        assert_eq!(strip_outer_parens("(0)"), "0");
    }

    #[test]
    fn strip_outer_parens_with_whitespace() {
        assert_eq!(strip_outer_parens("( 0 )"), "0");
    }

    #[test]
    fn strip_outer_parens_multi_level() {
        assert_eq!(strip_outer_parens("((0))"), "0");
        assert_eq!(strip_outer_parens("(((undef)))"), "undef");
    }

    #[test]
    fn strip_outer_parens_does_not_strip_sibling_groups() {
        // "(a)(b)" — outer `(` closes at position 2, not at the trailing `)`;
        // must not strip.
        assert_eq!(strip_outer_parens("(a)(b)"), "(a)(b)");
    }

    #[test]
    fn strip_outer_parens_empty_parens() {
        assert_eq!(strip_outer_parens("()"), "");
    }

    // --- is_always_false depth guard (#795) ---

    #[test]
    fn always_false_300_levels_deep_zero() {
        // 300 nested parens around `0`.  This MUST complete without stack
        // overflow now that strip_outer_parens is iterative.
        let depth = 300usize;
        let s = format!("{}0{}", "(".repeat(depth), ")".repeat(depth));
        assert!(is_always_false(&s));
    }

    #[test]
    fn always_false_300_levels_deep_one_is_not_false() {
        let depth = 300usize;
        let s = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        assert!(!is_always_false(&s));
    }

    #[test]
    fn always_false_300_levels_deep_variable_is_not_false() {
        let depth = 300usize;
        let s = format!("{}$x{}", "(".repeat(depth), ")".repeat(depth));
        assert!(!is_always_false(&s));
    }

    // --- is_always_true depth guard (#795) ---

    #[test]
    fn always_true_300_levels_deep_one() {
        let depth = 300usize;
        let s = format!("{}1{}", "(".repeat(depth), ")".repeat(depth));
        assert!(is_always_true(&s));
    }

    #[test]
    fn always_true_300_levels_deep_zero_is_not_true() {
        let depth = 300usize;
        let s = format!("{}0{}", "(".repeat(depth), ")".repeat(depth));
        assert!(!is_always_true(&s));
    }

    #[test]
    fn always_true_300_levels_deep_variable_is_not_true() {
        let depth = 300usize;
        let s = format!("{}$x{}", "(".repeat(depth), ")".repeat(depth));
        assert!(!is_always_true(&s));
    }
}
