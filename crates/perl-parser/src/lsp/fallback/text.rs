//! Text-based fallback implementations
//!
//! Provides fallback implementations for LSP features when full AST analysis
//! is unavailable or fails.

use crate::lsp::utils::byte_to_utf16_col;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::json;

lazy_static! {
    /// Matches package declarations: `package Foo::Bar`
    static ref PACKAGE_RE: Regex = Regex::new(r"^\s*package\s+([\w:]+)").unwrap();
    /// Matches subroutine definitions: `sub foo`
    static ref SUB_RE: Regex = Regex::new(r"^\s*sub\s+(\w+)").unwrap();
}

/// Extract code lenses from text when AST parsing fails
pub fn extract_text_based_code_lenses(
    text: &str,
    _uri: &str,
) -> Vec<crate::code_lens_provider::CodeLens> {
    let mut lenses = Vec::new();

    // Find package declarations
    for (line_num, line) in text.lines().enumerate() {
        if let Some(captures) = PACKAGE_RE.captures(line) {
            if let Some(pkg_name) = captures.get(1) {
                let name = pkg_name.as_str().to_string();

                lenses.push(crate::code_lens_provider::CodeLens {
                    range: crate::code_lens_provider::Range {
                        start: crate::code_lens_provider::Position {
                            line: line_num as u32,
                            character: byte_to_utf16_col(line, pkg_name.start()) as u32,
                        },
                        end: crate::code_lens_provider::Position {
                            line: line_num as u32,
                            character: byte_to_utf16_col(line, pkg_name.end()) as u32,
                        },
                    },
                    command: None, // Will be resolved later
                    data: Some(json!({
                        "name": name,
                        "kind": "package"
                    })),
                });
            }
        }
    }

    // Find subroutine declarations
    for (line_num, line) in text.lines().enumerate() {
        if let Some(captures) = SUB_RE.captures(line) {
            if let Some(sub_name) = captures.get(1) {
                let name = sub_name.as_str().to_string();

                lenses.push(crate::code_lens_provider::CodeLens {
                    range: crate::code_lens_provider::Range {
                        start: crate::code_lens_provider::Position {
                            line: line_num as u32,
                            character: byte_to_utf16_col(line, sub_name.start()) as u32,
                        },
                        end: crate::code_lens_provider::Position {
                            line: line_num as u32,
                            character: byte_to_utf16_col(line, sub_name.end()) as u32,
                        },
                    },
                    command: None, // Will be resolved later
                    data: Some(json!({
                        "name": name,
                        "kind": "subroutine"
                    })),
                });
            }
        }
    }

    lenses
}

/// Extract symbols from text when AST parsing fails
#[cfg(feature = "workspace")]
pub fn extract_text_based_symbols(
    text: &str,
    uri: &str,
    query: &str,
) -> Vec<crate::workspace_index::LspWorkspaceSymbol> {
    use crate::workspace_index::{LspLocation, LspPosition, LspRange, LspWorkspaceSymbol};

    let mut symbols = Vec::new();
    let query_lower = query.to_lowercase();

    // Find subroutine definitions
    for (line_num, line) in text.lines().enumerate() {
        if let Some(captures) = SUB_RE.captures(line) {
            if let Some(sub_name) = captures.get(1) {
                let name = sub_name.as_str().to_string();
                if name.to_lowercase().contains(&query_lower) {
                    symbols.push(LspWorkspaceSymbol {
                        name,
                        kind: 12, // Function
                        location: LspLocation {
                            uri: uri.to_string(),
                            range: LspRange {
                                start: LspPosition {
                                    line: line_num as u32,
                                    character: byte_to_utf16_col(line, sub_name.start()) as u32,
                                },
                                end: LspPosition {
                                    line: line_num as u32,
                                    character: byte_to_utf16_col(line, sub_name.end()) as u32,
                                },
                            },
                        },
                        container_name: None,
                    });
                }
            }
        }
    }

    // Find package declarations
    for (line_num, line) in text.lines().enumerate() {
        if let Some(captures) = PACKAGE_RE.captures(line) {
            if let Some(pkg_name) = captures.get(1) {
                let name = pkg_name.as_str().to_string();
                if name.to_lowercase().contains(&query_lower) {
                    symbols.push(LspWorkspaceSymbol {
                        name,
                        kind: 4, // Namespace
                        location: LspLocation {
                            uri: uri.to_string(),
                            range: LspRange {
                                start: LspPosition {
                                    line: line_num as u32,
                                    character: byte_to_utf16_col(line, pkg_name.start()) as u32,
                                },
                                end: LspPosition {
                                    line: line_num as u32,
                                    character: byte_to_utf16_col(line, pkg_name.end()) as u32,
                                },
                            },
                        },
                        container_name: None,
                    });
                }
            }
        }
    }

    symbols
}

/// Extract folding ranges from text with brace-depth awareness
///
/// This function properly handles nested blocks inside subroutines by tracking
/// brace depth. A subroutine's closing brace is only matched when the depth
/// returns to the level it was at when the subroutine started.
///
/// # Example
/// ```text
/// sub foo {          # depth 0 -> 1, push (line, 0)
///     if (1) {       # depth 1 -> 2
///     }              # depth 2 -> 1 (not sub's depth, no pop)
/// }                  # depth 1 -> 0 (matches sub's depth, pop and emit)
/// ```
pub fn folding_ranges_from_text(src: &str, limit: usize) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();

    // Track subroutines with (start_line, depth_at_start)
    let mut sub_stack: Vec<(usize, i32)> = Vec::new();
    let mut pod_start: Option<usize> = None;
    let mut brace_depth: i32 = 0;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();

        // Skip lines that look like strings (basic heuristic)
        if trimmed.starts_with('"') || trimmed.starts_with('\'') || trimmed.starts_with('`') {
            continue;
        }

        // POD documentation blocks
        if trimmed.starts_with("=pod") || trimmed.starts_with("=head") {
            pod_start = Some(i);
        } else if trimmed.starts_with("=cut") {
            if let Some(start) = pod_start.take() {
                if i > start {
                    out.push(serde_json::json!({
                        "startLine": start as u32,
                        "endLine": i as u32,
                        "kind": "comment"
                    }));
                }
            }
        }

        // Count braces in this line (outside of strings/comments - best effort)
        let (opens, closes) = count_braces_in_line(trimmed);

        // Subroutine blocks - record starting depth before the open brace
        if trimmed.starts_with("sub ") && opens > 0 {
            sub_stack.push((i, brace_depth));
        }

        // Update brace depth
        brace_depth += opens as i32;
        brace_depth -= closes as i32;

        // Check if we've returned to a subroutine's starting depth
        if closes > 0 && pod_start.is_none() {
            // Check each pending sub to see if we've closed it
            while let Some(&(start, start_depth)) = sub_stack.last() {
                if brace_depth <= start_depth {
                    sub_stack.pop();
                    if i > start {
                        out.push(serde_json::json!({
                            "startLine": start as u32,
                            "endLine": i as u32,
                            "kind": "region"
                        }));
                    }
                } else {
                    break;
                }
            }
        }
    }

    if out.len() > limit {
        out.truncate(limit);
    }
    out
}

/// Count opening and closing braces in a line, attempting to skip strings
fn count_braces_in_line(line: &str) -> (usize, usize) {
    let mut opens = 0;
    let mut closes = 0;
    let mut in_single = false;
    let mut in_double = false;
    let bytes = line.as_bytes();

    for i in 0..bytes.len() {
        let b = bytes[i];
        let escaped = i > 0 && bytes[i - 1] == b'\\';

        if in_single {
            if b == b'\'' && !escaped {
                in_single = false;
            }
        } else if in_double {
            if b == b'"' && !escaped {
                in_double = false;
            }
        } else {
            match b {
                b'\'' => in_single = true,
                b'"' => in_double = true,
                b'#' => break, // Comment - stop counting
                b'{' => opens += 1,
                b'}' => closes += 1,
                _ => {}
            }
        }
    }

    (opens, closes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folding_single_sub() {
        let src = "sub foo {\n    my $x = 1;\n}\n";
        let ranges = folding_ranges_from_text(src, 100);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0]["startLine"], 0);
        assert_eq!(ranges[0]["endLine"], 2);
    }

    #[test]
    fn test_folding_nested_blocks() {
        // Regression test: nested blocks should not prematurely close the sub
        let src = r#"sub foo {
    if (1) {
        print "hello";
    }
    for my $i (1..10) {
        print $i;
    }
}
"#;
        let ranges = folding_ranges_from_text(src, 100);
        // Should have exactly one folding range for the sub
        assert_eq!(ranges.len(), 1, "Expected 1 folding range, got {:?}", ranges);
        assert_eq!(ranges[0]["startLine"], 0);
        assert_eq!(ranges[0]["endLine"], 7); // The closing brace of sub foo
    }

    #[test]
    fn test_folding_multiple_subs() {
        let src = r#"sub foo {
    my $x = 1;
}

sub bar {
    my $y = 2;
}
"#;
        let ranges = folding_ranges_from_text(src, 100);
        assert_eq!(ranges.len(), 2, "Expected 2 folding ranges, got {:?}", ranges);
        // First sub
        assert_eq!(ranges[0]["startLine"], 0);
        assert_eq!(ranges[0]["endLine"], 2);
        // Second sub
        assert_eq!(ranges[1]["startLine"], 4);
        assert_eq!(ranges[1]["endLine"], 6);
    }

    #[test]
    fn test_folding_pod_sections() {
        let src = r#"=pod

This is documentation.

=cut

sub foo {
    my $x = 1;
}
"#;
        let ranges = folding_ranges_from_text(src, 100);
        assert_eq!(ranges.len(), 2, "Expected 2 folding ranges (POD + sub), got {:?}", ranges);
        // POD section
        assert_eq!(ranges[0]["kind"], "comment");
        // Sub
        assert_eq!(ranges[1]["kind"], "region");
    }

    #[test]
    fn test_folding_braces_in_strings_ignored() {
        let src = r#"sub foo {
    my $x = "a { string } with braces";
    print $x;
}
"#;
        let ranges = folding_ranges_from_text(src, 100);
        assert_eq!(ranges.len(), 1, "Expected 1 folding range, got {:?}", ranges);
        assert_eq!(ranges[0]["startLine"], 0);
        assert_eq!(ranges[0]["endLine"], 3); // Line 3 is the closing brace
    }

    #[test]
    fn test_count_braces_basic() {
        assert_eq!(count_braces_in_line("sub foo {"), (1, 0));
        assert_eq!(count_braces_in_line("}"), (0, 1));
        assert_eq!(count_braces_in_line("{ }"), (1, 1));
        assert_eq!(count_braces_in_line("{{ }}"), (2, 2));
    }

    #[test]
    fn test_count_braces_in_strings() {
        // Braces inside strings should be ignored
        assert_eq!(count_braces_in_line(r#"my $x = "{";"#), (0, 0));
        assert_eq!(count_braces_in_line(r#"my $x = '}';"#), (0, 0));
    }

    #[test]
    fn test_count_braces_in_comments() {
        // Braces after # should be ignored
        assert_eq!(count_braces_in_line("my $x = 1; # { comment"), (0, 0));
    }
}
