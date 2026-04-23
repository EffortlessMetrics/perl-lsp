//! Rename application logic
//!
//! This module provides methods for applying rename edits.

use perl_parser_core::SourceLocation;
use perl_semantic_analyzer::symbol::SymbolKind;

use super::types::{RenameOptions, TextEdit};

/// Adjust location to exclude sigil
pub fn adjust_location_for_sigil(mut location: SourceLocation, kind: SymbolKind) -> SourceLocation {
    if let Some(sigil) = kind.sigil() {
        // Skip the sigil character
        location.start += sigil.len();
    }
    location
}

/// Find occurrences in comments and strings
pub fn find_occurrences_in_text(
    name: &str,
    kind: SymbolKind,
    options: &RenameOptions,
    source: &str,
) -> Vec<TextEdit> {
    let mut edits = Vec::new();

    // Build search pattern
    let pattern = if let Some(sigil) = kind.sigil() {
        format!("{}{}", sigil, name)
    } else {
        name.to_string()
    };

    // Search through the source
    let mut search_pos = 0;
    while let Some(pos) = source[search_pos..].find(&pattern) {
        let absolute_pos = search_pos + pos;

        // Check if this is in a comment or string
        let in_comment = is_in_comment(absolute_pos, source);
        let in_string = is_in_string(absolute_pos, source);

        if (in_comment && options.rename_in_comments) || (in_string && options.rename_in_strings) {
            // Make sure it's a whole word
            let before_ok = absolute_pos == 0
                || source.chars().nth(absolute_pos - 1).is_none_or(|c| !c.is_alphanumeric());
            let after_pos = absolute_pos + pattern.len();
            let after_ok = after_pos >= source.len()
                || source.chars().nth(after_pos).is_none_or(|c| !c.is_alphanumeric());

            if before_ok && after_ok {
                let start = if let Some(sigil) = kind.sigil() {
                    absolute_pos + sigil.len()
                } else {
                    absolute_pos
                };

                edits.push(TextEdit {
                    location: SourceLocation { start, end: start + name.len() },
                    new_text: name.to_string(),
                });
            }
        }

        search_pos = absolute_pos + 1;
    }

    edits
}

/// Check if position is in a comment
pub fn is_in_comment(position: usize, source: &str) -> bool {
    let line_start =
        if position == 0 { 0 } else { source[..position].rfind('\n').map_or(0, |p| p + 1) };
    let line = &source[line_start..];

    if let Some(comment_pos) = line.find('#') {
        let comment_absolute = line_start + comment_pos;
        position >= comment_absolute
    } else {
        false
    }
}

/// Check if position is in a string
pub fn is_in_string(position: usize, source: &str) -> bool {
    // Simple heuristic - count quotes before position
    let before = &source[..position];
    let single_quotes = before.matches('\'').count();
    let double_quotes = before.matches('"').count();

    single_quotes % 2 == 1 || double_quotes % 2 == 1
}

/// Apply rename edits to source text
#[allow(dead_code)]
pub fn apply_rename_edits(source: &str, edits: &[TextEdit]) -> String {
    let mut result = source.to_string();

    // Apply edits in reverse order to maintain positions
    for edit in edits.iter().rev() {
        let start = edit.location.start;
        let end = edit.location.end;

        if start <= result.len() && end <= result.len() && start <= end {
            result.replace_range(start..end, &edit.new_text);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use perl_parser_core::SourceLocation;
    use perl_semantic_analyzer::symbol::{SymbolKind, VarKind};

    use super::{
        RenameOptions, TextEdit, adjust_location_for_sigil, apply_rename_edits,
        find_occurrences_in_text, is_in_comment, is_in_string,
    };

    #[test]
    fn adjust_location_for_sigil_skips_sigils_for_variables() -> Result<(), Box<dyn Error>> {
        let location = SourceLocation { start: 10, end: 14 };
        let adjusted = adjust_location_for_sigil(location, SymbolKind::Variable(VarKind::Scalar));
        assert_eq!(adjusted.start, 11);
        assert_eq!(adjusted.end, 14);
        Ok(())
    }

    #[test]
    fn comment_and_string_detection_respects_line_and_quote_boundaries()
    -> Result<(), Box<dyn Error>> {
        let source = "my $x = 1;\n# comment with $x\nmy $s = \"$x\";\n";
        let declaration_pos = source.find("$x =").ok_or("missing declaration")?;
        let comment_pos = source.find("# comment with $x").ok_or("missing comment")? + 15;
        let string_pos = source.rfind("$x\"").ok_or("missing string use")?;

        assert!(!is_in_comment(declaration_pos, source));
        assert!(is_in_comment(comment_pos, source));
        assert!(!is_in_string(declaration_pos, source));
        assert!(is_in_string(string_pos, source));
        Ok(())
    }

    #[test]
    fn find_occurrences_honors_comment_and_string_options() -> Result<(), Box<dyn Error>> {
        let source = "my $x = 1;\n# rename $x in comment\nmy $s = \"$x\";\n";
        let options = RenameOptions {
            rename_in_comments: true,
            rename_in_strings: true,
            validate_new_name: true,
        };

        let edits =
            find_occurrences_in_text("x", SymbolKind::Variable(VarKind::Scalar), &options, source);
        assert_eq!(edits.len(), 2, "expected one comment and one string occurrence");
        assert!(edits.iter().all(|edit| edit.new_text == "x"));
        Ok(())
    }

    #[test]
    fn find_occurrences_matches_whole_words_only() -> Result<(), Box<dyn Error>> {
        let source = "# $x $xy $x2\n\"$x and $xy\"\n";
        let options = RenameOptions {
            rename_in_comments: true,
            rename_in_strings: true,
            validate_new_name: true,
        };

        let edits =
            find_occurrences_in_text("x", SymbolKind::Variable(VarKind::Scalar), &options, source);
        assert_eq!(edits.len(), 2, "only standalone $x should match");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_applies_in_reverse_order() -> Result<(), Box<dyn Error>> {
        let source = "my $x = $x + 1;";
        let edits = vec![
            TextEdit {
                location: SourceLocation { start: 4, end: 5 },
                new_text: "value".to_string(),
            },
            TextEdit {
                location: SourceLocation { start: 9, end: 10 },
                new_text: "value".to_string(),
            },
        ];

        let renamed = apply_rename_edits(source, &edits);
        assert_eq!(renamed, "my $value = $value + 1;");
        Ok(())
    }


    // ============ Green TDD Edge Case Tests - Part 1 ============
    // These tests verify boundary conditions and error paths

    #[test]
    fn adjust_location_for_sigil_array() -> Result<(), Box<dyn Error>> {
        let location = SourceLocation { start: 20, end: 30 };
        let adjusted = adjust_location_for_sigil(location, SymbolKind::Variable(VarKind::Array));
        assert_eq!(adjusted.start, 21);
        Ok(())
    }

    #[test]
    fn adjust_location_for_sigil_hash() -> Result<(), Box<dyn Error>> {
        let location = SourceLocation { start: 5, end: 10 };
        let adjusted = adjust_location_for_sigil(location, SymbolKind::Variable(VarKind::Hash));
        assert_eq!(adjusted.start, 6);
        Ok(())
    }

    #[test]
    fn adjust_location_for_sigil_no_sigil() -> Result<(), Box<dyn Error>> {
        let location = SourceLocation { start: 10, end: 20 };
        let adjusted = adjust_location_for_sigil(location, SymbolKind::Subroutine);
        assert_eq!(adjusted.start, 10);
        Ok(())
    }

    #[test]
    fn is_in_comment_empty_source() -> Result<(), Box<dyn Error>> {
        assert!(!is_in_comment(0, ""));
        Ok(())
    }

    #[test]
    fn is_in_comment_single_hash() -> Result<(), Box<dyn Error>> {
        assert!(is_in_comment(0, "#"));
        Ok(())
    }

    #[test]
    fn is_in_comment_multiline_no_hash_on_second_line() -> Result<(), Box<dyn Error>> {
        let source = "my $var\n$value";
        let pos = source.find("\n").ok_or("newline not found")? + 1;
        assert!(!is_in_comment(pos, source));
        Ok(())
    }

    #[test]
    fn is_in_string_empty_source() -> Result<(), Box<dyn Error>> {
        assert!(!is_in_string(0, ""));
        Ok(())
    }

    #[test]
    fn is_in_string_before_open_quote() -> Result<(), Box<dyn Error>> {
        assert!(!is_in_string(0, "'string'"));
        Ok(())
    }

    #[test]
    fn is_in_string_inside_quotes() -> Result<(), Box<dyn Error>> {
        assert!(is_in_string(1, "'string'"));
        Ok(())
    }

    #[test]
    fn find_occurrences_not_in_comment_or_string() -> Result<(), Box<dyn Error>> {
        let source = "my $x = 5;";
        let options = RenameOptions {
            rename_in_comments: true,
            rename_in_strings: true,
            validate_new_name: true,
        };
        let edits = find_occurrences_in_text("x", SymbolKind::Variable(VarKind::Scalar), &options, source);
        assert_eq!(edits.len(), 0);
        Ok(())
    }

    #[test]
    fn find_occurrences_comments_only() -> Result<(), Box<dyn Error>> {
        let source = "my $var = 1; # $var here";
        let options = RenameOptions {
            rename_in_comments: true,
            rename_in_strings: false,
            validate_new_name: true,
        };
        let edits = find_occurrences_in_text("var", SymbolKind::Variable(VarKind::Scalar), &options, source);
        assert_eq!(edits.len(), 1);
        Ok(())
    }

    #[test]
    fn find_occurrences_strings_only() -> Result<(), Box<dyn Error>> {
        let source = "my $var = \"$var\"; $var = 1;";
        let options = RenameOptions {
            rename_in_comments: false,
            rename_in_strings: true,
            validate_new_name: true,
        };
        let edits = find_occurrences_in_text("var", SymbolKind::Variable(VarKind::Scalar), &options, source);
        assert_eq!(edits.len(), 1);
        Ok(())
    }

    #[test]
    fn apply_rename_edits_empty_source_with_insert() -> Result<(), Box<dyn Error>> {
        let edits = vec![TextEdit {
            location: SourceLocation { start: 0, end: 0 },
            new_text: "text".to_string(),
        }];
        let result = apply_rename_edits("", &edits);
        assert_eq!(result, "text");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_entire_source_replace() -> Result<(), Box<dyn Error>> {
        let edits = vec![TextEdit {
            location: SourceLocation { start: 0, end: 3 },
            new_text: "new".to_string(),
        }];
        let result = apply_rename_edits("old", &edits);
        assert_eq!(result, "new");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_at_start_boundary() -> Result<(), Box<dyn Error>> {
        let edits = vec![TextEdit {
            location: SourceLocation { start: 0, end: 1 },
            new_text: "X".to_string(),
        }];
        let result = apply_rename_edits("abc", &edits);
        assert_eq!(result, "Xbc");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_at_end_boundary() -> Result<(), Box<dyn Error>> {
        let edits = vec![TextEdit {
            location: SourceLocation { start: 2, end: 3 },
            new_text: "X".to_string(),
        }];
        let result = apply_rename_edits("abc", &edits);
        assert_eq!(result, "abX");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_two_separated_edits() -> Result<(), Box<dyn Error>> {
        let edits = vec![
            TextEdit {
                location: SourceLocation { start: 0, end: 1 },
                new_text: "X".to_string(),
            },
            TextEdit {
                location: SourceLocation { start: 2, end: 3 },
                new_text: "Y".to_string(),
            },
        ];
        let result = apply_rename_edits("abcd", &edits);
        assert_eq!(result, "XbYd");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_expand_text() -> Result<(), Box<dyn Error>> {
        let edits = vec![TextEdit {
            location: SourceLocation { start: 0, end: 1 },
            new_text: "hello".to_string(),
        }];
        let result = apply_rename_edits("x", &edits);
        assert_eq!(result, "hello");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_shrink_text() -> Result<(), Box<dyn Error>> {
        let edits = vec![TextEdit {
            location: SourceLocation { start: 0, end: 5 },
            new_text: "hi".to_string(),
        }];
        let result = apply_rename_edits("hello", &edits);
        assert_eq!(result, "hi");
        Ok(())
    }

    #[test]
    fn apply_rename_edits_middle_replacement() -> Result<(), Box<dyn Error>> {
        let edits = vec![TextEdit {
            location: SourceLocation { start: 2, end: 4 },
            new_text: "XX".to_string(),
        }];
        let result = apply_rename_edits("abcde", &edits);
        assert_eq!(result, "abXXe");
        Ok(())
    }

    #[test]
    fn find_occurrences_all_options_disabled() -> Result<(), Box<dyn Error>> {
        let source = "my $x = 1; # $x comment \"$x\" string";
        let options = RenameOptions {
            rename_in_comments: false,
            rename_in_strings: false,
            validate_new_name: true,
        };
        let edits = find_occurrences_in_text("x", SymbolKind::Variable(VarKind::Scalar), &options, source);
        assert_eq!(edits.len(), 0);
        Ok(())
    }
}
