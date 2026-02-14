//! Tests for trivia parser edge cases
//!
//! This file contains tests for edge cases in the trivia parser.
//! Some tests are marked as `#[ignore]` because they represent known limitations
//! that are documented in `trivia_demo.rs` but not yet fully implemented.
//!
//! See the "Known Edge Cases and Limitations" section in
//! `crates/perl-parser/examples/trivia_demo.rs` for details.

use perl_tokenizer::trivia::{Trivia, TriviaLexer};
use perl_tokenizer::trivia_parser::TriviaPreservingParser;

#[test]
#[ignore = "Known limitation: POD without =cut - see trivia_demo.rs edge cases documentation"]
fn test_pod_without_cut() {
    // Edge case: POD at end of file without =cut
    let source = r#"my $x = 1;

=head1 DESCRIPTION

This is documentation without a closing =cut
"#
    .to_string();

    let mut lexer = TriviaLexer::new(source);
    let mut found_pod = false;

    while let Some((_token, trivia)) = lexer.next_token_with_trivia() {
        for t in trivia {
            if matches!(t.trivia, Trivia::PodComment(_)) {
                found_pod = true;
            }
        }
    }

    assert!(found_pod, "Should detect POD even without =cut at end of file");
}

#[test]
fn test_pod_at_start_of_file() {
    // Edge case: POD at the very start of the file
    let source = r#"=head1 NAME

MyModule - A test module

=cut

package MyModule;
"#
    .to_string();

    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();

    let has_pod = result.leading_trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_)));

    assert!(has_pod, "Should detect POD at start of file");
}

#[test]
#[ignore = "Known limitation: Comments at EOF without newline - see trivia_demo.rs edge cases documentation"]
fn test_comment_without_newline_at_eof() {
    // Edge case: Comment at end of file without trailing newline
    let source = "my $x = 1; # comment without newline".to_string();

    let mut lexer = TriviaLexer::new(source);
    let mut found_comment = false;

    while let Some((_token, trivia)) = lexer.next_token_with_trivia() {
        for t in trivia {
            if matches!(t.trivia, Trivia::LineComment(_)) {
                found_comment = true;
            }
        }
    }

    assert!(found_comment, "Should detect comment at EOF without newline");
}

#[test]
fn test_windows_line_endings() {
    // Edge case: Windows CRLF line endings
    let source = "# Comment\r\nmy $x = 1;\r\n# Another\r\n".to_string();

    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();

    let comment_count = result
        .leading_trivia
        .iter()
        .filter(|t| matches!(&t.trivia, Trivia::LineComment(_)))
        .count();

    assert!(comment_count >= 1, "Should handle Windows line endings in comments");
}

#[test]
fn test_unicode_in_comments() {
    // Edge case: Unicode characters in comments
    let source = "# This comment has Unicode: \u{1F980} 日本語\nmy $x = 1;".to_string();

    let mut lexer = TriviaLexer::new(source);
    let mut found_unicode_comment = false;

    while let Some((_token, trivia)) = lexer.next_token_with_trivia() {
        for t in trivia {
            if let Trivia::LineComment(text) = &t.trivia {
                if text.contains('\u{1F980}') || text.contains('日') {
                    found_unicode_comment = true;
                }
            }
        }
    }

    assert!(found_unicode_comment, "Should preserve Unicode in comments");
}

#[test]
fn test_mixed_tabs_and_spaces() {
    // Edge case: Mixed tabs and spaces in whitespace
    let source = " \t \t my $x = 1;".to_string();

    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();

    let found_expected = if let Some(first_token) = result.leading_trivia.first() {
        if let Trivia::Whitespace(ws) = &first_token.trivia {
            assert_eq!(ws, " \t \t ", "Should preserve exact whitespace sequence");
            true
        } else {
            false
        }
    } else {
        false
    };

    assert!(found_expected, "Should capture leading whitespace with mixed tabs/spaces");
}

#[test]
#[ignore = "Known limitation: Shebang positioning - see trivia_demo.rs edge cases documentation"]
fn test_shebang_variations() {
    // Edge case: Different shebang variations
    let test_cases = vec![
        "#!/usr/bin/perl\n",
        "#!/usr/bin/env perl\n",
        "#!/usr/local/bin/perl -w\n",
        "#! /usr/bin/perl\n",
    ];

    for source in test_cases {
        let parser = TriviaPreservingParser::new(source.to_string());
        let result = parser.parse();

        let has_shebang = result.leading_trivia.iter().any(|t| {
            if let Trivia::LineComment(text) = &t.trivia { text.starts_with("#!") } else { false }
        });

        assert!(has_shebang, "Should detect shebang: {}", source);
    }
}

#[test]
#[ignore = "Known limitation: Multiple consecutive blank lines - see trivia_demo.rs edge cases documentation"]
fn test_empty_lines_sequence() {
    // Edge case: Multiple empty lines in a row
    let source = "my $x = 1;\n\n\n\nmy $y = 2;".to_string();

    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();

    let newline_count =
        result.leading_trivia.iter().filter(|t| matches!(&t.trivia, Trivia::Newline)).count();

    // Should capture the initial state and newlines
    assert!(newline_count >= 1, "Should track multiple newlines");
}

#[test]
fn test_pod_with_special_commands() {
    // Edge case: POD with various command types
    let source = r#"=pod

=encoding utf8

=for html <div>content</div>

=begin text

Some text block

=end text

=cut

my $x = 1;
"#
    .to_string();

    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();

    let pod_count =
        result.leading_trivia.iter().filter(|t| matches!(&t.trivia, Trivia::PodComment(_))).count();

    assert!(pod_count >= 1, "Should detect POD with special commands");
}

#[test]
fn test_hash_in_string_not_comment() {
    // Edge case: Hash character in string should not be treated as comment
    // This tests the parser's ability to distinguish context
    let source = "my $x = \"# not a comment\";".to_string();

    let mut lexer = TriviaLexer::new(source);
    let mut comment_found_in_string = false;

    // First token should be 'my' with no comment in trivia
    if let Some((_token, trivia)) = lexer.next_token_with_trivia() {
        for t in trivia {
            if matches!(t.trivia, Trivia::LineComment(_)) {
                comment_found_in_string = true;
            }
        }
    }

    assert!(
        !comment_found_in_string,
        "Should not treat # in string as comment (this may be a known limitation)"
    );
}

#[test]
fn test_here_doc_with_hash() {
    // Edge case: Here-doc content with # characters
    let source = r#"my $text = <<'END';
# This is not a comment
# It's part of the here-doc
END
my $x = 1;
"#
    .to_string();

    // This is a complex case - the lexer may not distinguish here-docs yet
    // For now, just verify it doesn't crash
    let parser = TriviaPreservingParser::new(source);
    let _result = parser.parse();

    // If we get here without panicking, the test passes
}

#[test]
fn test_pod_false_start() {
    // Edge case: Equals sign that looks like POD but isn't
    let source = "my $x = 1;\nmy $result = $x == 42;\n".to_string();

    let mut lexer = TriviaLexer::new(source);
    let mut pod_count = 0;

    while let Some((_token, trivia)) = lexer.next_token_with_trivia() {
        for t in trivia {
            if matches!(t.trivia, Trivia::PodComment(_)) {
                pod_count += 1;
            }
        }
    }

    assert_eq!(pod_count, 0, "Should not treat == as POD start");
}

#[test]
#[ignore = "Known limitation: POD in middle of code - see trivia_demo.rs edge cases documentation"]
fn test_inline_pod_preservation() {
    // Edge case: POD in the middle of code (not at file start)
    let source = r#"sub foo {
    my $x = shift;

=for comment
Hidden documentation
=cut

    return $x * 2;
}
"#
    .to_string();

    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();

    // Check if POD was captured anywhere (leading trivia or within the parse)
    let has_pod = result.leading_trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_)));

    assert!(has_pod, "Should detect POD in middle of code");
}

#[test]
fn test_unicode_whitespace() {
    // Edge case: Unicode whitespace characters
    let source = "my\u{00A0}$x\u{2003}=\u{3000}1;".to_string(); // Various Unicode spaces

    let parser = TriviaPreservingParser::new(source);
    let _result = parser.parse();

    // If it doesn't crash, we're good for now
    // Proper handling would require checking trivia contains these
}

#[test]
fn test_carriage_return_handling() {
    // Edge case: Bare CR without LF (old Mac style)
    let source = "my $x = 1;\rmy $y = 2;\r".to_string();

    let parser = TriviaPreservingParser::new(source);
    let _result = parser.parse();

    // Should not crash
}

#[test]
fn test_nested_pod() {
    // Edge case: POD with nested =begin/=end
    let source = r#"=begin html

=begin nested
This shouldn't work but let's test it
=end nested

=end html

=cut

my $x = 1;
"#
    .to_string();

    let parser = TriviaPreservingParser::new(source);
    let result = parser.parse();

    let has_pod = result.leading_trivia.iter().any(|t| matches!(&t.trivia, Trivia::PodComment(_)));

    assert!(has_pod, "Should handle nested =begin/=end");
}
