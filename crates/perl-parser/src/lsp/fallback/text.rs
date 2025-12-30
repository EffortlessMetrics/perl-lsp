//! Text-based fallback implementations
//!
//! Provides fallback implementations for LSP features when full AST analysis
//! is unavailable or fails.

use serde_json::json;
use crate::lsp::utils::byte_to_utf16_col;

/// Extract code lenses from text when AST parsing fails
pub fn extract_text_based_code_lenses(
    text: &str,
    _uri: &str,
) -> Vec<crate::code_lens_provider::CodeLens> {
    let mut lenses = Vec::new();

    // Use simple regex patterns to find common Perl constructs that should have code lenses
    use regex::Regex;

    // Find package declarations
    if let Ok(pkg_regex) = Regex::new(r"^\s*package\s+([\w:]+)") {
        for (line_num, line) in text.lines().enumerate() {
            if let Some(captures) = pkg_regex.captures(line) {
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
    }

    // Find subroutine declarations
    if let Ok(sub_regex) = Regex::new(r"^\s*sub\s+(\w+)") {
        for (line_num, line) in text.lines().enumerate() {
            if let Some(captures) = sub_regex.captures(line) {
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
    }

    lenses
}

/// Extract symbols from text when AST parsing fails
pub fn extract_text_based_symbols(
    text: &str,
    uri: &str,
    query: &str,
) -> Vec<crate::workspace_index::LspWorkspaceSymbol> {
    let mut symbols = Vec::new();
    let query_lower = query.to_lowercase();

    // Use simple regex patterns to find common Perl symbols
    use regex::Regex;
    use crate::workspace_index::{LspWorkspaceSymbol, LspLocation, LspRange, LspPosition};

    // Find subroutine definitions
    if let Ok(sub_regex) = Regex::new(r"^\s*sub\s+(\w+)") {
        for (line_num, line) in text.lines().enumerate() {
            if let Some(captures) = sub_regex.captures(line) {
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
                                        character: byte_to_utf16_col(line, sub_name.start())
                                            as u32,
                                    },
                                    end: LspPosition {
                                        line: line_num as u32,
                                        character: byte_to_utf16_col(line, sub_name.end())
                                            as u32,
                                    },
                                },
                            },
                            container_name: None,
                        });
                    }
                }
            }
        }
    }

    // Find package declarations
    if let Ok(pkg_regex) = Regex::new(r"^\s*package\s+([\w:]+)") {
        for (line_num, line) in text.lines().enumerate() {
            if let Some(captures) = pkg_regex.captures(line) {
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
                                        character: byte_to_utf16_col(line, pkg_name.start())
                                            as u32,
                                    },
                                    end: LspPosition {
                                        line: line_num as u32,
                                        character: byte_to_utf16_col(line, pkg_name.end())
                                            as u32,
                                    },
                                },
                            },
                            container_name: None,
                        });
                    }
                }
            }
        }
    }

    symbols
}

pub fn folding_ranges_from_text(src: &str, limit: usize) -> Vec<serde_json::Value> {
    let mut out = Vec::new();
    let lines: Vec<&str> = src.lines().collect();

    // Track different types of blocks
    let mut sub_stack: Vec<usize> = Vec::new();
    let mut pod_start: Option<usize> = None;

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

        // Subroutine blocks
        if trimmed.starts_with("sub ") && trimmed.contains('{') {
            sub_stack.push(i);
        } else if trimmed.starts_with('}') && pod_start.is_none() {
            if let Some(start) = sub_stack.pop() {
                if i > start {
                    out.push(serde_json::json!({
                        "startLine": start as u32,
                        "endLine": i as u32,
                        "kind": "region"
                    }));
                }
            }
        }
    }

    if out.len() > limit {
        out.truncate(limit);
    }
    out
}
