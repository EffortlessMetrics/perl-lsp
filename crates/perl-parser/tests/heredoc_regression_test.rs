use perl_parser::Parser;

#[test]
fn test_heredoc_body_consumption() {
    // Test that heredoc bodies are consumed, not parsed as tokens
    let input = r#"my $x = <<END;
hello
world
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Should NOT contain "hello" or "world" as identifiers
    assert!(!sexp.contains("(identifier hello)"));
    assert!(!sexp.contains("(identifier world)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_multiple_heredocs_single_line() {
    // Test multiple heredocs on one line (FIFO processing)
    let input = r#"print <<FIRST, <<SECOND;
body of first
FIRST
body of second
SECOND
say "done";"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Bodies should be consumed, not parsed as identifiers
    assert!(!sexp.contains("(identifier body)"));
    assert!(!sexp.contains("(identifier of)"));
    assert!(!sexp.contains("(identifier first)"));
    assert!(!sexp.contains("(identifier second)"));
    assert!(sexp.contains(r#"say ((string_interpolated "\"done\""#));
}

#[test]
fn test_three_heredocs() {
    let input = r#"print <<A, <<B, <<C;
aaa
A
bbb
B
ccc
C
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // All three bodies should be consumed
    assert!(!sexp.contains("(identifier aaa)"));
    assert!(!sexp.contains("(identifier bbb)"));
    assert!(!sexp.contains("(identifier ccc)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_indented_heredoc() {
    // Test Perl 5.26+ indented heredoc (<<~)
    let input = r#"my $x = <<~END;
    hello
    world
  END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Body should be consumed
    assert!(!sexp.contains("(identifier hello)"));
    assert!(!sexp.contains("(identifier world)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_mixed_heredocs() {
    // Test mix of regular and indented heredocs
    let input = r#"print <<REGULAR, <<~INDENTED;
not indented
REGULAR
    indented content
  INDENTED
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Bodies should be consumed
    assert!(!sexp.contains("(identifier not)"));
    assert!(!sexp.contains("(identifier indented)"));
    assert!(!sexp.contains("(identifier content)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_heredoc_fake_data_marker() {
    // Test that __DATA__ inside heredoc is ignored
    let input = r#"my $x = <<END;
__DATA__
not real data
END
say 1;
__DATA__
real data"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Fake __DATA__ in heredoc should be consumed
    assert!(!sexp.contains("(identifier not)"));
    assert!(!sexp.contains("(identifier real)"), "Should not parse 'real' from heredoc body");
    // Real __DATA__ section should exist
    assert!(sexp.contains("data_section"));
}

#[test]
fn test_non_indented_heredoc_rejects_indented_terminator() {
    // Non-indented heredoc should NOT accept indented terminator
    let input = r#"my $x = <<END;
hello
  END
more content"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Should get UNKNOWN_REST since terminator wasn't found
    assert!(sexp.contains("UNKNOWN_REST"));
}

#[test]
fn test_quoted_label_with_spaces() {
    let input = r#"my $x = <<'END THIS';
foo bar
END THIS
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Body should be consumed
    assert!(!sexp.contains("(identifier foo)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_backslashed_label_no_interpolation() {
    let input = r#"my $x = <<\END;
$var should not interpolate
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Body consumed, no interpolation check needed at lexer level
    assert!(!sexp.contains("(identifier var)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_whitespace_after_heredoc_operator() {
    let input = r#"my $x = <<   "END";
hello world
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Body should be consumed
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_crlf_line_endings() {
    let input = "my $x = <<END;\r\nhello\r\nEND\r\nsay 1;\r\n";

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Should work identically to LF endings
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_tab_indented_tilde_heredoc() {
    let input = "my $x = <<~END;\n\t\thello\n\t\tEND\nsay 1;";

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Tab-indented terminator should work with <<~
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_terminator_with_trailing_junk() {
    let input = r#"my $x = <<END;
hello
END junk after terminator
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Terminator with trailing non-space should not be recognized
    // Should get UNKNOWN_REST since terminator wasn't found
    assert!(sexp.contains("UNKNOWN_REST"));
}

#[test]
fn test_mixed_regular_and_tilde_heredocs() {
    let input = r#"print <<END, <<~INDENTED;
regular
END
    indented content
    INDENTED
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Both bodies should be consumed
    assert!(!sexp.contains("(identifier regular)"));
    assert!(!sexp.contains("(identifier indented)"));
    assert!(!sexp.contains("(identifier content)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_heredoc_after_large_regex() {
    // Test that budget limits don't interfere with heredoc handling
    let input = r#"m{.{1000}}; # Large regex
my $x = <<END;
hello
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // Heredoc should still work after large regex
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
}

#[test]
fn test_bom_at_file_start() {
    // UTF-8 BOM: EF BB BF
    let input = "\u{FEFF}my $x = <<END;\nhello\nEND\nsay 1;";

    let mut parser = Parser::new(input);
    let tree = parser.parse().unwrap();
    let sexp = tree.to_sexp();

    // BOM should be skipped, heredoc should work
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
}
