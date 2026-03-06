//! On-type formatting provider for Perl LSP.
//!
//! Provides automatic indentation and formatting when typing trigger characters.

use serde_json::Value;
use serde_json::json;

/// Computes on-type formatting edits for a Perl document based on character input.
///
/// Handles special characters (`{`, `}`, `;`, newlines) to provide automatic indentation
/// and formatting adjustments. Returns a vector of text edits to apply, or `None` if no
/// edits are needed for the given character.
pub fn compute_on_type_edit(text: &str, line: u32, col: u32, ch: char) -> Option<Vec<Value>> {
    let lines: Vec<&str> = text.lines().collect();

    if line as usize >= lines.len() {
        return None;
    }

    match ch {
        '{' => {
            let current_indent = get_indentation(lines[line as usize]);
            let new_indent = current_indent + 2;

            Some(vec![json!({
                "range": {
                    "start": {"line": line, "character": col},
                    "end": {"line": line, "character": col}
                },
                "newText": format!("\n{}", " ".repeat(new_indent))
            })])
        }
        '}' => {
            if line > 0 {
                let current_line = lines[line as usize];
                let current_indent = get_indentation(current_line);

                let target_indent = find_matching_brace_indent(&lines, line as usize)
                    .unwrap_or_else(|| current_indent.saturating_sub(2));

                if current_indent != target_indent {
                    Some(vec![json!({
                        "range": {
                            "start": {"line": line, "character": 0},
                            "end": {"line": line, "character": current_indent as u32}
                        },
                        "newText": " ".repeat(target_indent)
                    })])
                } else {
                    None
                }
            } else {
                None
            }
        }
        ';' => {
            let current_indent = get_indentation(lines[line as usize]);
            Some(vec![json!({
                "range": {
                    "start": {"line": line, "character": col},
                    "end": {"line": line, "character": col}
                },
                "newText": format!("\n{}", " ".repeat(current_indent))
            })])
        }
        '\n' | '\r' => {
            if line > 0 {
                let prev_line = lines[(line - 1) as usize];
                let prev_indent = get_indentation(prev_line);

                let trimmed = prev_line.trim_end();
                let indent = if trimmed.ends_with('{') { prev_indent + 2 } else { prev_indent };

                Some(vec![json!({
                    "range": {
                        "start": {"line": line, "character": 0},
                        "end": {"line": line, "character": 0}
                    },
                    "newText": " ".repeat(indent)
                })])
            } else {
                None
            }
        }
        _ => None,
    }
}

fn get_indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn find_matching_brace_indent(lines: &[&str], closing_line: usize) -> Option<usize> {
    let mut brace_count = 1;

    for i in (0..closing_line).rev() {
        let line = lines[i];
        for ch in line.chars().rev() {
            match ch {
                '}' => brace_count += 1,
                '{' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        return Some(get_indentation(line));
                    }
                }
                _ => {}
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::compute_on_type_edit;

    #[test]
    fn inserts_indent_after_open_brace() {
        let text = "if ($x) {";
        let edits = compute_on_type_edit(text, 0, 9, '{');
        assert!(edits.is_some());
    }

    #[test]
    fn returns_none_for_unknown_trigger() {
        let text = "my $x = 1;";
        let edits = compute_on_type_edit(text, 0, 5, 'a');
        assert!(edits.is_none());
    }

    #[test]
    fn out_of_bounds_line_is_none() {
        let edits = compute_on_type_edit("", 42, 0, '{');
        assert!(edits.is_none());
    }
}
