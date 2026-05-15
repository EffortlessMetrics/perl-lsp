use crate::{DeadCode, DeadCodeType};
use std::path::Path;

pub(crate) fn detect_dead_branches(file_path: &Path, text: &str, out: &mut Vec<DeadCode>) {
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();
    let mut i = 0;

    while i < n {
        let trimmed = lines[i].trim();

        let dead_reason_and_keyword: Option<(String, &str)> = 'detect: {
            for kw in &["if", "while", "elsif", "unless", "until", "for", "foreach"] {
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
                let after_cond = rest[condition.len() + 2..].trim();
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
    let c = condition.trim();
    // Perl considers a value false when it stringifies to "" or "0", so both
    // the bare number 0 and the quoted strings "0" / '0' are always false.
    matches!(c, "0" | "\"\"" | "''" | "\"0\"" | "'0'" | "undef")
        || (c.starts_with('(') && c.ends_with(')') && is_always_false(&c[1..c.len() - 1]))
}

fn is_always_true(condition: &str) -> bool {
    let c = condition.trim();
    if c.parse::<i64>().is_ok_and(|n| n != 0) {
        return true;
    }
    if c.parse::<f64>().is_ok_and(|n| n != 0.0) {
        return true;
    }
    if (c.starts_with('"') && c.ends_with('"') || c.starts_with('\'') && c.ends_with('\''))
        && c.len() > 2
    {
        let inner = &c[1..c.len() - 1];
        return inner != "0";
    }
    c.starts_with('(') && c.ends_with(')') && is_always_true(&c[1..c.len() - 1])
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
