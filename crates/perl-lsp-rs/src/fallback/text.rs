//! Text-based fallback implementations
//!
//! Provides fallback implementations for LSP features when full AST analysis
//! is unavailable or fails.

use crate::convert::{WirePosition, WireRange};
use crate::util::byte_to_utf16_col;
use regex::Regex;
use serde_json::json;
use std::sync::LazyLock;

/// Matches package declarations: `package Foo::Bar`
static PACKAGE_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^\s*package\s+([\w:]+)").ok());
/// Matches subroutine definitions: `sub foo`
static SUB_RE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"^\s*sub\s+(\w+)").ok());
/// Matches lexical/package variable declarations: `my $x`, `our @items`, `state %cache`
static VAR_DECL_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^\s*(?:my|our|state)\s+([\$\@\%]\w+)").ok());
/// Matches constant declarations: `use constant NAME => ...`
static CONSTANT_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^\s*use\s+constant\s+([A-Z_]\w*)\b").ok());

#[derive(Copy, Clone, Eq, PartialEq)]
enum TextSymbolKind {
    Package,
    Subroutine,
    Variable,
    Constant,
}

struct TextMatch {
    name: String,
    line: usize,
    start_byte: usize,
    end_byte: usize,
    kind: TextSymbolKind,
}

fn collect_text_matches(text: &str) -> Vec<TextMatch> {
    let mut matches = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        if let Some(captures) = PACKAGE_RE.as_ref().and_then(|re| re.captures(line))
            && let Some(package_name) = captures.get(1)
        {
            matches.push(TextMatch {
                name: package_name.as_str().to_string(),
                line: line_num,
                start_byte: package_name.start(),
                end_byte: package_name.end(),
                kind: TextSymbolKind::Package,
            });
        }

        if let Some(captures) = SUB_RE.as_ref().and_then(|re| re.captures(line))
            && let Some(sub_name) = captures.get(1)
        {
            matches.push(TextMatch {
                name: sub_name.as_str().to_string(),
                line: line_num,
                start_byte: sub_name.start(),
                end_byte: sub_name.end(),
                kind: TextSymbolKind::Subroutine,
            });
        }

        if let Some(captures) = VAR_DECL_RE.as_ref().and_then(|re| re.captures(line))
            && let Some(variable_name) = captures.get(1)
        {
            matches.push(TextMatch {
                name: variable_name.as_str().to_string(),
                line: line_num,
                start_byte: variable_name.start(),
                end_byte: variable_name.end(),
                kind: TextSymbolKind::Variable,
            });
        }

        if let Some(captures) = CONSTANT_RE.as_ref().and_then(|re| re.captures(line))
            && let Some(constant_name) = captures.get(1)
        {
            matches.push(TextMatch {
                name: constant_name.as_str().to_string(),
                line: line_num,
                start_byte: constant_name.start(),
                end_byte: constant_name.end(),
                kind: TextSymbolKind::Constant,
            });
        }
    }

    matches
}

/// Extract code lenses from text when AST parsing fails
pub fn extract_text_based_code_lenses(
    text: &str,
    _uri: &str,
) -> Vec<crate::code_lens_provider::CodeLens> {
    let mut lenses = Vec::new();
    let lines: Vec<&str> = text.lines().collect();

    for text_match in collect_text_matches(text) {
        let kind = match text_match.kind {
            TextSymbolKind::Package => "package",
            TextSymbolKind::Subroutine => "subroutine",
            TextSymbolKind::Variable | TextSymbolKind::Constant => continue,
        };
        let Some(line) = lines.get(text_match.line) else {
            continue;
        };

        lenses.push(crate::code_lens_provider::CodeLens {
            range: WireRange::new(
                WirePosition::new(
                    text_match.line as u32,
                    byte_to_utf16_col(line, text_match.start_byte) as u32,
                ),
                WirePosition::new(
                    text_match.line as u32,
                    byte_to_utf16_col(line, text_match.end_byte) as u32,
                ),
            ),
            command: None, // Will be resolved later
            data: Some(json!({
                "name": text_match.name,
                "kind": kind
            })),
        });
    }

    lenses
}

#[cfg(feature = "workspace")]
fn kind_to_symbol_code(kind: TextSymbolKind) -> u32 {
    match kind {
        TextSymbolKind::Package => 4,
        TextSymbolKind::Subroutine => 12,
        TextSymbolKind::Variable => 13,
        TextSymbolKind::Constant => 14,
    }
}

#[cfg(feature = "workspace")]
fn workspace_symbol_from_text_match(
    uri: &str,
    line_text: &str,
    text_match: TextMatch,
) -> crate::workspace_index::LspWorkspaceSymbol {
    use crate::workspace_index::LspWorkspaceSymbol;
    use perl_position_tracking::{WireLocation, WirePosition, WireRange};

    LspWorkspaceSymbol {
        name: text_match.name,
        kind: kind_to_symbol_code(text_match.kind),
        location: WireLocation::new(
            uri.to_string(),
            WireRange::new(
                WirePosition::new(
                    text_match.line as u32,
                    byte_to_utf16_col(line_text, text_match.start_byte) as u32,
                ),
                WirePosition::new(
                    text_match.line as u32,
                    byte_to_utf16_col(line_text, text_match.end_byte) as u32,
                ),
            ),
        ),
        container_name: None,
        workspace_folder_uri: None,
    }
}

/// Extract symbols from text when AST parsing fails
#[cfg(feature = "workspace")]
pub fn extract_text_based_symbols(
    text: &str,
    uri: &str,
    query: &str,
) -> Vec<crate::workspace_index::LspWorkspaceSymbol> {
    let mut symbols = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let query_lower = query.to_lowercase();

    for text_match in collect_text_matches(text) {
        if !text_match.name.to_lowercase().contains(&query_lower) {
            continue;
        }
        let Some(line_text) = lines.get(text_match.line) else {
            continue;
        };

        symbols.push(workspace_symbol_from_text_match(uri, line_text, text_match));
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
        if line_looks_like_string(trimmed) {
            continue;
        }

        // POD documentation blocks
        if line_starts_pod(trimmed) {
            pod_start = Some(i);
        } else if line_ends_pod(trimmed) {
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

fn line_looks_like_string(line: &str) -> bool {
    line.starts_with('"') || line.starts_with('\'') || line.starts_with('`')
}

fn line_starts_pod(line: &str) -> bool {
    line.starts_with("=pod") || line.starts_with("=head") || line.starts_with("=begin")
}

fn line_ends_pod(line: &str) -> bool {
    line.starts_with("=cut") || line.starts_with("=end")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_lens_fallback_extracts_only_package_and_subroutines() {
        let src = r#"
package My::Pkg;
my $local_var = 1;
use constant MAX_RETRY => 3;
sub helper {}
"#;
        let lenses = extract_text_based_code_lenses(src, "file:///test.pl");
        assert_eq!(lenses.len(), 2);

        let lens_data: Vec<serde_json::Value> =
            lenses.iter().map(|lens| lens.data.clone().unwrap_or_default()).collect();
        assert!(lens_data.iter().any(|data| data["kind"] == "package"));
        assert!(lens_data.iter().any(|data| data["kind"] == "subroutine"));
    }

    #[cfg(feature = "workspace")]
    #[test]
    fn test_workspace_symbol_fallback_extracts_variables_and_constants() {
        let src = r#"
my $local_var = 1;
our @global_items = ();
state %cache = ();
use constant MAX_RETRY => 3;
sub helper {}
"#;
        let symbols = extract_text_based_symbols(src, "file:///test.pl", "");
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();

        assert!(names.contains(&"$local_var"));
        assert!(names.contains(&"@global_items"));
        assert!(names.contains(&"%cache"));
        assert!(names.contains(&"MAX_RETRY"));
        assert!(names.contains(&"helper"));
    }

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
    fn test_folding_begin_end_pod_sections() {
        let src = r#"=begin comment
Generated docs
=end comment

sub foo {
    my $x = 1;
}
"#;
        let ranges = folding_ranges_from_text(src, 100);
        assert_eq!(ranges.len(), 2, "Expected 2 folding ranges (POD + sub), got {:?}", ranges);
        assert_eq!(ranges[0]["kind"], "comment");
        assert_eq!(ranges[0]["startLine"], 0);
        assert_eq!(ranges[0]["endLine"], 2);
        assert_eq!(ranges[1]["kind"], "region");
        assert_eq!(ranges[1]["startLine"], 4);
        assert_eq!(ranges[1]["endLine"], 6);
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

    #[test]
    fn test_folding_line_heuristics() {
        assert!(line_looks_like_string(r#""quoted""#));
        assert!(line_looks_like_string("'quoted'"));
        assert!(line_looks_like_string("`quoted`"));
        assert!(!line_looks_like_string("sub foo {"));

        assert!(line_starts_pod("=pod"));
        assert!(line_starts_pod("=head1 NAME"));
        assert!(line_starts_pod("=begin comment"));
        assert!(!line_starts_pod("=cut"));

        assert!(line_ends_pod("=cut"));
        assert!(line_ends_pod("=end comment"));
        assert!(!line_ends_pod("=pod"));
    }
}
