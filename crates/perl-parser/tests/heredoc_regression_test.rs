use perl_parser::Parser;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn test_heredoc_body_consumption() -> TestResult {
    // Test that heredoc bodies are consumed, not parsed as tokens
    let input = r#"my $x = <<END;
hello
world
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Should NOT contain "hello" or "world" as identifiers
    assert!(!sexp.contains("(identifier hello)"));
    assert!(!sexp.contains("(identifier world)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_multiple_heredocs_single_line() -> TestResult {
    // Test multiple heredocs on one line (FIFO processing)
    let input = r#"print <<FIRST, <<SECOND;
body of first
FIRST
body of second
SECOND
say "done";"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Bodies should be consumed, not parsed as identifiers
    assert!(!sexp.contains("(identifier body)"));
    assert!(!sexp.contains("(identifier of)"));
    assert!(!sexp.contains("(identifier first)"));
    assert!(!sexp.contains("(identifier second)"));
    // The 'say "done"' statement should be parsed as a function call
    assert!(sexp.contains("(call say"));
    assert!(sexp.contains("string_interpolated"));
    Ok(())
}

#[test]
fn test_three_heredocs() -> TestResult {
    let input = r#"print <<A, <<B, <<C;
aaa
A
bbb
B
ccc
C
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // All three bodies should be consumed
    assert!(!sexp.contains("(identifier aaa)"));
    assert!(!sexp.contains("(identifier bbb)"));
    assert!(!sexp.contains("(identifier ccc)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_indented_heredoc() -> TestResult {
    // Test Perl 5.26+ indented heredoc (<<~)
    let input = r#"my $x = <<~END;
    hello
    world
  END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Body should be consumed
    assert!(!sexp.contains("(identifier hello)"));
    assert!(!sexp.contains("(identifier world)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_mixed_heredocs() -> TestResult {
    // Test mix of regular and indented heredocs
    let input = r#"print <<REGULAR, <<~INDENTED;
not indented
REGULAR
    indented content
  INDENTED
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Bodies should be consumed
    assert!(!sexp.contains("(identifier not)"));
    assert!(!sexp.contains("(identifier indented)"));
    assert!(!sexp.contains("(identifier content)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_heredoc_fake_data_marker() -> TestResult {
    // Test that __DATA__ inside heredoc is ignored
    let input = r#"my $x = <<END;
__DATA__
not real data
END
say 1;
__DATA__
real data"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Fake __DATA__ in heredoc should be consumed
    assert!(!sexp.contains("(identifier not)"));
    assert!(!sexp.contains("(identifier real)"), "Should not parse 'real' from heredoc body");
    // Real __DATA__ section should exist
    assert!(sexp.contains("data_section"));
    Ok(())
}

#[test]
fn test_non_indented_heredoc_rejects_indented_terminator() -> TestResult {
    // Non-indented heredoc should NOT accept indented terminator
    let input = r#"my $x = <<END;
hello
  END
more content"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Should get UNKNOWN_REST since terminator wasn't found
    assert!(sexp.contains("UNKNOWN_REST"));
    Ok(())
}

#[test]
fn test_quoted_label_with_spaces() -> TestResult {
    let input = r#"my $x = <<'END THIS';
foo bar
END THIS
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Body should be consumed
    assert!(!sexp.contains("(identifier foo)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_backslashed_label_no_interpolation() -> TestResult {
    let input = r#"my $x = <<\END;
$var should not interpolate
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Body consumed, no interpolation check needed at lexer level
    assert!(!sexp.contains("(identifier var)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_whitespace_after_heredoc_operator() -> TestResult {
    let input = r#"my $x = <<   "END";
hello world
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Body should be consumed
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_crlf_line_endings() -> TestResult {
    let input = "my $x = <<END;\r\nhello\r\nEND\r\nsay 1;\r\n";

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Should work identically to LF endings
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_tab_indented_tilde_heredoc() -> TestResult {
    let input = "my $x = <<~END;\n\t\thello\n\t\tEND\nsay 1;";

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Tab-indented terminator should work with <<~
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_terminator_with_trailing_junk() -> TestResult {
    let input = r#"my $x = <<END;
hello
END junk after terminator
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Terminator with trailing non-space should not be recognized
    // Should get UNKNOWN_REST since terminator wasn't found
    assert!(sexp.contains("UNKNOWN_REST"));
    Ok(())
}

#[test]
fn test_mixed_regular_and_tilde_heredocs() -> TestResult {
    let input = r#"print <<END, <<~INDENTED;
regular
END
    indented content
    INDENTED
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Both bodies should be consumed
    assert!(!sexp.contains("(identifier regular)"));
    assert!(!sexp.contains("(identifier indented)"));
    assert!(!sexp.contains("(identifier content)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_heredoc_after_large_regex() -> TestResult {
    // Test that budget limits don't interfere with heredoc handling
    let input = r#"m{.{1000}}; # Large regex
my $x = <<END;
hello
END
say 1;"#;

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // Heredoc should still work after large regex
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_bom_at_file_start() -> TestResult {
    // UTF-8 BOM: EF BB BF
    let input = "\u{FEFF}my $x = <<END;\nhello\nEND\nsay 1;";

    let mut parser = Parser::new(input);
    let tree = parser.parse()?;
    let sexp = tree.to_sexp();

    // BOM should be skipped, heredoc should work
    assert!(!sexp.contains("(identifier hello)"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_comprehensive_crlf_tilde_trailing_spaces() -> TestResult {
    // Mixed CRLF + <<~ + trailing spaces on terminator
    let input = "my $x = <<~END;\r\n    indented\r\n    END  \t \r\nsay 1;\r\n";

    let mut parser = Parser::new(input);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();

    // Verify structure is preserved with CRLF
    assert!(sexp.contains("(my_declaration"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_data_end_with_trailing_junk_non_ws() -> TestResult {
    // __DATA__ and __END__ should reject lines with non-whitespace trailing junk
    let inputs = vec![
        "__DATA__ # comment\nShould not be data",
        "__DATA__abc\nShould not be data",
        "__END__ some text\nShould not be data",
        "__END__\\n\nShould not be data",  // Backslash after __END__
        "__DATA__123\nShould not be data", // Numbers after __DATA__
    ];

    for input in inputs {
        let mut parser = Parser::new(input);
        let ast = parser.parse()?;
        let sexp = ast.to_sexp();
        // Should not parse as data sections
        assert!(
            !sexp.contains("(data_section"),
            "Input '{}' incorrectly parsed as data section",
            input
        );
    }
    Ok(())
}

#[test]
fn test_reject_ruby_style_heredoc() -> TestResult {
    // Ruby uses <<- for indented heredocs, Perl does not
    // In Perl, <<-END is invalid syntax since << in expression context
    // without a left operand expects a heredoc label, but -END isn't valid.
    let input = "my $x = <<-END;\n    indented\n    -END\nsay 1;";

    let mut parser = Parser::new(input);
    let ast = parser.parse();

    // The parser should recover and continue parsing subsequent statements
    if let Ok(ast) = ast {
        let sexp = ast.to_sexp();
        // Should NOT produce a heredoc (verify heredoc body isn't captured)
        assert!(!sexp.contains("heredoc_interpolated"));
        assert!(!sexp.contains("heredoc_literal"));
        // The content after should be parsed as regular code
        // "say 1;" should still be parsed correctly
        assert!(sexp.contains("say"));
    }
    // If parse fails completely, that's also acceptable for invalid syntax
    Ok(())
}

#[test]
fn test_old_mac_cr_only_line_endings() -> TestResult {
    // Old Mac used \r only for line endings
    let input = "my $x = <<END;\rLine1\rLine2\rEND\rsay 1;\r";

    let mut parser = Parser::new(input);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    // Verify structure is preserved with CR-only line endings
    assert!(sexp.contains("(my_declaration"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_tilde_heredoc_allows_blank_lines() -> TestResult {
    // Perl allows empty lines in <<~ heredocs
    let input = "my $x = <<~END;\n    line1\n    \n    line3\n    END\nsay 1;\n";

    let mut parser = Parser::new(input);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    // Blank lines are allowed in indented heredocs
    assert!(sexp.contains("(my_declaration"));
    assert!(sexp.contains("say"));
    Ok(())
}

#[test]
fn test_end_with_trailing_junk_is_ignored() -> TestResult {
    // __END__ with non-whitespace trailing text should not be treated as data section
    let input = "__END__ trailing\nstill code\n";

    let mut parser = Parser::new(input);
    let ast = parser.parse()?;
    let sexp = ast.to_sexp();
    // Should not parse as data section
    assert!(
        !sexp.contains("(data_section"),
        "__END__ with trailing junk should not be data section"
    );
    Ok(())
}
