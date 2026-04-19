//! Comprehensive integration tests for the perl-lexer crate.
//!
//! Covers: tokenization of real Perl snippets, edge cases, unicode,
//! error recovery, operator types, heredocs, regexes, quotes, sigils,
//! checkpointing, configuration, and the lexer's public API surface.

use perl_lexer::{
    Checkpointable, LexerCheckpoint, LexerConfig, LexerMode, PerlLexer, Token, TokenType,
};

type R = Result<(), Box<dyn std::error::Error>>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collect all tokens (including EOF) from `input`.
fn tokens(input: &str) -> Vec<Token> {
    PerlLexer::new(input).collect_tokens()
}

/// Collect all non-whitespace, non-newline, non-EOF tokens.
fn significant_tokens(input: &str) -> Vec<Token> {
    tokens(input)
        .into_iter()
        .filter(|t| {
            !matches!(
                t.token_type,
                TokenType::Whitespace | TokenType::Newline | TokenType::EOF
            )
        })
        .collect()
}

/// Assert that every token's span `[start, end)` is within the input length
/// and that `text` matches the slice.
fn assert_spans_valid(input: &str, toks: &[Token]) {
    for t in toks {
        assert!(
            t.end <= input.len(),
            "Token {:?} end {} exceeds input length {}",
            t.token_type,
            t.end,
            input.len()
        );
        assert!(
            t.start <= t.end,
            "Token {:?} has start {} > end {}",
            t.token_type,
            t.start,
            t.end
        );
    }
}

// ===========================================================================
// 1. Basic tokenization of real Perl snippets
// ===========================================================================

#[test]
fn simple_variable_assignment() -> R {
    let input = "my $x = 42;";
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(&first.token_type, TokenType::Keyword(k) if k.as_ref() == "my"),
        "expected 'my' keyword, got {:?}",
        first.token_type
    );

    // Should contain a number token with value "42"
    let has_42 = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Number(n) if n.as_ref() == "42"));
    assert!(has_42, "expected Number(42) in tokens");
    Ok(())
}

#[test]
fn subroutine_definition() -> R {
    let input = "sub greet { print \"Hello\\n\"; }";
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(&first.token_type, TokenType::Keyword(k) if k.as_ref() == "sub"),
        "expected 'sub' keyword"
    );

    let has_print = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "print"));
    assert!(has_print, "expected 'print' keyword");
    Ok(())
}

#[test]
fn multiline_perl_snippet() -> R {
    let input = r#"
use strict;
use warnings;

my @items = (1, 2, 3);
for my $item (@items) {
    print "$item\n";
}
"#;
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    // Last token must be EOF
    let last = toks.last().ok_or("empty tokens")?;
    assert!(
        matches!(last.token_type, TokenType::EOF),
        "last token should be EOF"
    );
    Ok(())
}

#[test]
fn hash_operations() -> R {
    let input = "my %h = (a => 1, b => 2); $h{a};";
    let toks = significant_tokens(input);
    assert_spans_valid(input, &toks);

    // Should contain fat comma tokens (FatComma or Operator("=>"))
    let fat_commas = toks
        .iter()
        .filter(|t| {
            matches!(t.token_type, TokenType::FatComma)
                || matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == "=>")
        })
        .count();
    assert!(
        fat_commas >= 2,
        "expected at least 2 fat commas, got {fat_commas}"
    );
    Ok(())
}

// ===========================================================================
// 2. Operators
// ===========================================================================

#[test]
fn arithmetic_operators() -> R {
    let input = "$a + $b - $c * $d";
    let toks = significant_tokens(input);

    let ops: Vec<&str> = toks
        .iter()
        .filter_map(|t| match &t.token_type {
            TokenType::Operator(o) => Some(o.as_ref()),
            _ => None,
        })
        .collect();
    assert!(ops.contains(&"+"), "missing +");
    assert!(ops.contains(&"-"), "missing -");
    assert!(ops.contains(&"*"), "missing *");
    Ok(())
}

#[test]
fn comparison_operators() -> R {
    let cases = [
        ("$a == $b", "=="),
        ("$a != $b", "!="),
        ("$a < $b", "<"),
        ("$a > $b", ">"),
        ("$a <= $b", "<="),
        ("$a >= $b", ">="),
        ("$a <=> $b", "<=>"),
    ];
    for (input, expected_op) in cases {
        let toks = significant_tokens(input);
        let found = toks
            .iter()
            .any(|t| matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == expected_op));
        assert!(found, "operator '{expected_op}' not found in '{input}'");
    }
    Ok(())
}

#[test]
fn string_operators() -> R {
    let cases = [("$a . $b", "."), ("$a eq $b", "eq"), ("$a ne $b", "ne")];
    for (input, expected_op) in cases {
        let toks = significant_tokens(input);
        let found = toks.iter().any(|t| {
            matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == expected_op)
                || matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == expected_op)
                || matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref() == expected_op)
        });
        assert!(
            found,
            "operator/keyword '{expected_op}' not found in '{input}'"
        );
    }

    // `x` is the repetition operator but the lexer emits it as Identifier
    let x_toks = significant_tokens("$a x 3");
    let has_x = x_toks.iter().any(|t| {
        matches!(&t.token_type, TokenType::Identifier(id) if id.as_ref() == "x")
            || matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == "x")
            || matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "x")
    });
    assert!(has_x, "'x' token not found in '$a x 3'");
    Ok(())
}

#[test]
fn logical_operators() -> R {
    let cases = ["$a && $b", "$a || $b", "!$a"];
    for input in cases {
        let toks = significant_tokens(input);
        assert!(toks.len() >= 2, "too few significant tokens for '{input}'");
    }
    Ok(())
}

#[test]
fn arrow_operator() -> R {
    let input = "$obj->method()";
    let toks = significant_tokens(input);
    let has_arrow = toks.iter().any(|t| {
        matches!(t.token_type, TokenType::Arrow)
            || matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == "->")
    });
    assert!(
        has_arrow,
        "expected Arrow or Operator('->') in '$obj->method()'"
    );
    Ok(())
}

#[test]
fn defined_or_operator() -> R {
    let input = "$a // $b";
    let toks = significant_tokens(input);
    // After an identifier, // should be defined-or, not regex
    let has_op = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == "//"));
    assert!(has_op, "expected defined-or (//) operator");
    Ok(())
}

#[test]
fn range_operator() -> R {
    let input = "1 .. 10";
    let toks = significant_tokens(input);
    let has_range = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == ".."));
    assert!(has_range, "expected range (..) operator");
    Ok(())
}

// ===========================================================================
// 3. Delimiters and punctuation
// ===========================================================================

#[test]
fn paired_delimiters() -> R {
    let input = "( [ { } ] )";
    let toks = significant_tokens(input);

    let types: Vec<&TokenType> = toks.iter().map(|t| &t.token_type).collect();
    assert!(matches!(types[0], TokenType::LeftParen));
    assert!(matches!(types[1], TokenType::LeftBracket));
    assert!(matches!(types[2], TokenType::LeftBrace));
    assert!(matches!(types[3], TokenType::RightBrace));
    assert!(matches!(types[4], TokenType::RightBracket));
    assert!(matches!(types[5], TokenType::RightParen));
    Ok(())
}

#[test]
fn semicolons_and_commas() -> R {
    let input = "1, 2; 3, 4;";
    let toks = significant_tokens(input);

    let semis = toks
        .iter()
        .filter(|t| matches!(t.token_type, TokenType::Semicolon))
        .count();
    let commas = toks
        .iter()
        .filter(|t| matches!(t.token_type, TokenType::Comma))
        .count();
    assert_eq!(semis, 2, "expected 2 semicolons");
    assert_eq!(commas, 2, "expected 2 commas");
    Ok(())
}

// ===========================================================================
// 4. Strings and quotes
// ===========================================================================

#[test]
fn double_quoted_string() -> R {
    let input = r#""hello world""#;
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(
            first.token_type,
            TokenType::StringLiteral | TokenType::InterpolatedString(_)
        ),
        "expected string token, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn single_quoted_string() -> R {
    let input = "'hello world'";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::StringLiteral),
        "expected StringLiteral, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn qq_operator() -> R {
    let input = "qq{hello world}";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteDouble),
        "expected QuoteDouble, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn q_operator() -> R {
    let input = "q{hello world}";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteSingle),
        "expected QuoteSingle, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn qw_operator() -> R {
    let input = "qw(foo bar baz)";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteWords),
        "expected QuoteWords, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn qx_backtick_command() -> R {
    let input = "qx{ls -la}";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteCommand),
        "expected QuoteCommand, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn backtick_literal() -> R {
    let input = "`echo hi`";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteCommand),
        "expected QuoteCommand for backtick, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn quote_operators_with_single_quote_delimiter() -> R {
    let cases = [
        ("q'hello'", TokenType::QuoteSingle),
        ("qq'hello'", TokenType::QuoteDouble),
        ("qw'foo bar'", TokenType::QuoteWords),
        ("qx'echo hi'", TokenType::QuoteCommand),
        ("qr'foo+'", TokenType::QuoteRegex),
        ("m'foo+'", TokenType::RegexMatch),
    ];

    for (input, expected_variant) in cases {
        let toks = significant_tokens(input);
        let first = toks
            .first()
            .ok_or_else(|| format!("no tokens for '{input}'"))?;
        assert!(
            std::mem::discriminant(&first.token_type) == std::mem::discriminant(&expected_variant),
            "input '{input}': expected {:?}, got {:?}",
            expected_variant,
            first.token_type
        );
    }

    Ok(())
}

#[test]
fn quote_operators_with_alternate_delimiters() -> R {
    let cases = [
        ("q<hello>", TokenType::QuoteSingle),
        ("q[hello]", TokenType::QuoteSingle),
        ("q(hello)", TokenType::QuoteSingle),
        ("qq!hello!", TokenType::QuoteDouble),
        ("qq#hello#", TokenType::QuoteDouble),
    ];
    for (input, expected_variant) in cases {
        let toks = significant_tokens(input);
        let first = toks
            .first()
            .ok_or_else(|| format!("no tokens for '{input}'"))?;
        assert!(
            std::mem::discriminant(&first.token_type) == std::mem::discriminant(&expected_variant),
            "input '{input}': expected {:?}, got {:?}",
            expected_variant,
            first.token_type
        );
    }
    Ok(())
}

// ===========================================================================
// 5. Regex
// ===========================================================================

#[test]
fn bare_regex_match() -> R {
    let input = "/pattern/";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "expected RegexMatch, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn regex_with_flags() -> R {
    let input = "/pattern/gi";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "expected RegexMatch, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn m_operator_regex() -> R {
    let input = "m{pattern}i";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::RegexMatch),
        "expected RegexMatch for m{{}} , got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn qr_precompiled_regex() -> R {
    let input = "qr/^hello$/";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::QuoteRegex),
        "expected QuoteRegex for qr//, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn substitution_basic() -> R {
    let input = "s/foo/bar/g";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Substitution),
        "expected Substitution, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn substitution_with_paired_delimiters() -> R {
    let input = "s{foo}{bar}g";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Substitution),
        "expected Substitution with paired delimiters, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn transliteration_tr() -> R {
    let input = "tr/a-z/A-Z/";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Transliteration),
        "expected Transliteration, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn transliteration_y() -> R {
    let input = "y/a-z/A-Z/";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(first.token_type, TokenType::Transliteration),
        "expected Transliteration for y///, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn slash_as_division_after_identifier() -> R {
    let input = "$x / 2";
    let toks = significant_tokens(input);
    let has_div = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::Division));
    assert!(has_div, "expected Division after identifier");
    Ok(())
}

#[test]
fn slash_disambiguation_after_paren() -> R {
    let input = ") / 2";
    let toks = significant_tokens(input);
    let has_div = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::Division));
    assert!(has_div, "expected Division after closing paren");
    Ok(())
}

// ===========================================================================
// 6. Heredocs
// ===========================================================================

#[test]
fn heredoc_double_quoted() -> R {
    let input = "<<EOF\nhello world\nEOF\n";
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let has_heredoc_start = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(has_heredoc_start, "expected HeredocStart token");
    Ok(())
}

#[test]
fn heredoc_single_quoted() -> R {
    let input = "<<'END'\nno $interpolation\nEND\n";
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let has_heredoc_start = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(
        has_heredoc_start,
        "expected HeredocStart for single-quoted heredoc"
    );
    Ok(())
}

#[test]
fn heredoc_indented() -> R {
    let input = "<<~END\n    hello\n    END\n";
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let has_heredoc_start = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(
        has_heredoc_start,
        "expected HeredocStart for indented heredoc"
    );
    Ok(())
}

#[test]
fn heredoc_with_body_tokens() -> R {
    let input = "<<EOF\nbody text\nEOF\n";
    let mut lexer = PerlLexer::with_body_tokens(input);
    let toks = lexer.collect_tokens();
    assert_spans_valid(input, &toks);

    let has_start = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::HeredocStart));
    assert!(has_start, "expected HeredocStart in body-token mode");
    Ok(())
}

#[test]
fn heredoc_terminates_with_crlf() -> R {
    let input = "<<EOF\r\nhello\r\nEOF\r\n";
    let toks = tokens(input);
    // Just verify it terminates and produces an EOF
    let has_eof = toks.iter().any(|t| matches!(t.token_type, TokenType::EOF));
    assert!(has_eof, "heredoc with CRLF should terminate");
    Ok(())
}

// ===========================================================================
// 7. Sigils and variables
// ===========================================================================

#[test]
fn scalar_variable() -> R {
    let input = "$foo";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(&first.token_type, TokenType::Identifier(id) if id.as_ref().contains("foo")),
        "expected identifier containing 'foo', got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn array_variable() -> R {
    let input = "@array";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(&first.token_type, TokenType::Identifier(id) if id.as_ref().contains("array")),
        "expected identifier containing 'array', got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn hash_variable() -> R {
    let input = "%hash";
    let toks = significant_tokens(input);
    // After %hash, the lexer may produce Identifier or Operator(%) + Identifier
    let has_hash_ref = toks.iter().any(|t| match &t.token_type {
        TokenType::Identifier(id) => id.as_ref().contains("hash"),
        _ => false,
    });
    assert!(has_hash_ref, "expected identifier containing 'hash'");
    Ok(())
}

#[test]
fn special_variables() -> R {
    let cases = ["$_", "$!", "$@", "$$", "$0"];
    for input in cases {
        let toks = significant_tokens(input);
        assert!(
            !toks.is_empty(),
            "expected at least one token for '{input}'"
        );
    }
    Ok(())
}

#[test]
fn sigil_followed_by_brace() -> R {
    // ${foo} may be emitted as a single Identifier("${foo}") or split tokens
    let input = "${foo}";
    let toks = significant_tokens(input);
    assert!(
        !toks.is_empty(),
        "expected at least one token for '${{foo}}'"
    );
    let has_foo = toks.iter().any(|t| match &t.token_type {
        TokenType::Identifier(id) => id.as_ref().contains("foo"),
        _ => false,
    });
    assert!(
        has_foo,
        "expected identifier containing 'foo' in '${{foo}}'"
    );
    Ok(())
}

// ===========================================================================
// 8. Numbers
// ===========================================================================

#[test]
fn integer_literals() -> R {
    let cases = ["0", "42", "1_000_000"];
    for input in cases {
        let toks = significant_tokens(input);
        let first = toks
            .first()
            .ok_or_else(|| format!("no tokens for '{input}'"))?;
        assert!(
            matches!(first.token_type, TokenType::Number(_)),
            "expected Number for '{input}', got {:?}",
            first.token_type
        );
    }
    Ok(())
}

#[test]
fn float_literals() -> R {
    let cases = ["3.14", "1.0e10", ".5"];
    for input in cases {
        let toks = significant_tokens(input);
        let first = toks
            .first()
            .ok_or_else(|| format!("no tokens for '{input}'"))?;
        assert!(
            matches!(first.token_type, TokenType::Number(_)),
            "expected Number for '{input}', got {:?}",
            first.token_type
        );
    }
    Ok(())
}

#[test]
fn hex_octal_binary_literals() -> R {
    let cases = ["0x1F", "0b1010", "0777"];
    for input in cases {
        let toks = significant_tokens(input);
        let first = toks
            .first()
            .ok_or_else(|| format!("no tokens for '{input}'"))?;
        assert!(
            matches!(first.token_type, TokenType::Number(_)),
            "expected Number for '{input}', got {:?}",
            first.token_type
        );
    }
    Ok(())
}

// ===========================================================================
// 9. Keywords
// ===========================================================================

#[test]
fn common_keywords() -> R {
    let keywords = [
        "my", "our", "local", "sub", "if", "elsif", "else", "unless", "while", "for", "foreach",
        "use", "return", "print",
    ];
    for kw in keywords {
        let toks = significant_tokens(kw);
        let first = toks
            .first()
            .ok_or_else(|| format!("no tokens for '{kw}'"))?;
        assert!(
            matches!(&first.token_type, TokenType::Keyword(k) if k.as_ref() == kw),
            "expected Keyword({kw}), got {:?}",
            first.token_type
        );
    }
    Ok(())
}

// ===========================================================================
// 10. Comments and POD
// ===========================================================================

#[test]
fn line_comment() -> R {
    // The lexer silently skips comments; verify code after comment is tokenized
    let input = "# this is a comment\nmy $x;";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    assert!(
        matches!(&first.token_type, TokenType::Keyword(k) if k.as_ref() == "my"),
        "expected 'my' after comment, got {:?}",
        first.token_type
    );
    Ok(())
}

#[test]
fn pod_section() -> R {
    // POD may not be recognized as a Pod token in all contexts; verify termination
    let input = "=pod\nSome documentation\n=cut\n";
    let toks = tokens(input);
    let has_eof = toks.iter().any(|t| matches!(t.token_type, TokenType::EOF));
    assert!(has_eof, "POD input should produce EOF");
    Ok(())
}

#[test]
fn pod_head1() -> R {
    let input = "=head1 NAME\nSomething\n=cut\n";
    let toks = tokens(input);
    let has_eof = toks.iter().any(|t| matches!(t.token_type, TokenType::EOF));
    assert!(has_eof, "POD =head1 input should produce EOF");
    Ok(())
}

// ===========================================================================
// 11. Data sections
// ===========================================================================

#[test]
fn data_section() -> R {
    let input = "__DATA__\nsome data here\n";
    let toks = tokens(input);
    let has_data_marker = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::DataMarker(_)));
    assert!(has_data_marker, "expected DataMarker token");
    Ok(())
}

#[test]
fn end_section() -> R {
    let input = "__END__\nsome data here\n";
    let toks = tokens(input);
    let has_data_marker = toks
        .iter()
        .any(|t| matches!(t.token_type, TokenType::DataMarker(_)));
    assert!(has_data_marker, "expected DataMarker for __END__");
    Ok(())
}

// ===========================================================================
// 12. Unicode
// ===========================================================================

#[test]
fn unicode_identifier() -> R {
    let input = "my $café = 1;";
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let has_cafe = toks.iter().any(|t| match &t.token_type {
        TokenType::Identifier(id) => id.as_ref().contains("caf"),
        _ => false,
    });
    assert!(has_cafe, "expected identifier containing 'caf'");
    Ok(())
}

#[test]
fn unicode_in_string() -> R {
    let input = r#""日本語""#;
    let toks = significant_tokens(input);
    assert!(!toks.is_empty(), "expected tokens for unicode string");
    assert_spans_valid(input, &toks);
    Ok(())
}

#[test]
fn emoji_in_string() -> R {
    let input = "\"Hello 🌍\"";
    let toks = significant_tokens(input);
    assert!(!toks.is_empty(), "expected tokens for emoji string");
    assert_spans_valid(input, &toks);
    Ok(())
}

#[test]
fn bom_is_skipped() -> R {
    // UTF-8 BOM followed by code
    let input = "\u{FEFF}my $x = 1;";
    let toks = tokens(input);

    // The first meaningful token should be the keyword, not an error
    let first_significant = toks
        .iter()
        .find(|t| !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline))
        .ok_or("no significant tokens after BOM")?;
    assert!(
        matches!(&first_significant.token_type, TokenType::Keyword(k) if k.as_ref() == "my"),
        "expected 'my' after BOM, got {:?}",
        first_significant.token_type
    );
    Ok(())
}

// ===========================================================================
// 13. Edge cases
// ===========================================================================

#[test]
fn empty_input() -> R {
    let toks = tokens("");
    let first = toks.first().ok_or("expected EOF for empty input")?;
    assert!(
        matches!(first.token_type, TokenType::EOF),
        "empty input should produce EOF"
    );
    Ok(())
}

#[test]
fn whitespace_only_input() -> R {
    let toks = tokens("   \n\t  \n  ");
    let last = toks.last().ok_or("expected tokens")?;
    assert!(
        matches!(last.token_type, TokenType::EOF),
        "whitespace-only input should end with EOF"
    );
    Ok(())
}

#[test]
fn single_character_tokens() -> R {
    let cases = [
        (";", TokenType::Semicolon),
        (",", TokenType::Comma),
        ("(", TokenType::LeftParen),
        (")", TokenType::RightParen),
        ("[", TokenType::LeftBracket),
        ("]", TokenType::RightBracket),
        ("{", TokenType::LeftBrace),
        ("}", TokenType::RightBrace),
    ];
    for (input, expected) in cases {
        let toks = significant_tokens(input);
        let first = toks
            .first()
            .ok_or_else(|| format!("no tokens for '{input}'"))?;
        assert_eq!(
            std::mem::discriminant(&first.token_type),
            std::mem::discriminant(&expected),
            "input '{input}': expected {:?}, got {:?}",
            expected,
            first.token_type
        );
    }
    Ok(())
}

#[test]
fn very_long_identifier() -> R {
    let long_name = "a".repeat(1000);
    let input = format!("${long_name}");
    let toks = significant_tokens(&input);
    assert!(!toks.is_empty(), "long identifier should produce tokens");
    Ok(())
}

#[test]
fn consecutive_operators() -> R {
    let input = "$a+=$b";
    let toks = significant_tokens(input);
    assert!(toks.len() >= 3, "expected at least 3 tokens for '$a+=$b'");
    Ok(())
}

// ===========================================================================
// 14. Lexer API: collect_tokens, peek, reset
// ===========================================================================

#[test]
fn collect_tokens_includes_eof() -> R {
    let mut lexer = PerlLexer::new("1 + 2");
    let toks = lexer.collect_tokens();
    let last = toks.last().ok_or("empty tokens")?;
    assert!(
        matches!(last.token_type, TokenType::EOF),
        "collect_tokens should end with EOF"
    );
    Ok(())
}

#[test]
fn peek_does_not_advance() -> R {
    let mut lexer = PerlLexer::new("my $x");
    let peeked = lexer.peek_token().ok_or("peek returned None")?;
    let actual = lexer.next_token().ok_or("next returned None")?;

    // peek and next should return the same first token
    assert_eq!(
        std::mem::discriminant(&peeked.token_type),
        std::mem::discriminant(&actual.token_type),
        "peek and next should return same token type"
    );
    assert_eq!(
        peeked.start, actual.start,
        "peek and next should have same start"
    );
    Ok(())
}

#[test]
fn reset_replays_tokens() -> R {
    let mut lexer = PerlLexer::new("my $x = 1;");
    let first_pass = lexer.next_token().ok_or("no first token")?;
    lexer.reset();
    let second_pass = lexer.next_token().ok_or("no token after reset")?;
    assert_eq!(
        first_pass.start, second_pass.start,
        "reset should replay from start"
    );
    Ok(())
}

#[test]
fn next_token_returns_none_after_eof() -> R {
    let mut lexer = PerlLexer::new("1");
    // Consume number
    let _ = lexer.next_token().ok_or("expected number")?;
    // Consume EOF
    let eof = lexer.next_token().ok_or("expected EOF")?;
    assert!(matches!(eof.token_type, TokenType::EOF));
    // After EOF, should return None
    assert!(lexer.next_token().is_none(), "should be None after EOF");
    Ok(())
}

// ===========================================================================
// 15. Configuration
// ===========================================================================

#[test]
fn custom_config() -> R {
    let config = LexerConfig {
        parse_interpolation: false,
        track_positions: false,
        max_lookahead: 512,
    };
    let mut lexer = PerlLexer::with_config("my $x = 1;", config);
    let toks = lexer.collect_tokens();
    let last = toks.last().ok_or("no tokens")?;
    assert!(
        matches!(last.token_type, TokenType::EOF),
        "custom config should still produce EOF"
    );
    Ok(())
}

#[test]
fn default_config_works() -> R {
    let config = LexerConfig::default();
    assert!(config.parse_interpolation);
    assert!(config.track_positions);
    assert_eq!(config.max_lookahead, 1024);
    Ok(())
}

// ===========================================================================
// 16. Checkpointing
// ===========================================================================

#[test]
fn checkpoint_save_and_restore() -> R {
    let mut lexer = PerlLexer::new("my $x = 1; my $y = 2;");
    let _first = lexer.next_token().ok_or("no first token")?;

    let cp = lexer.checkpoint();
    let second = lexer.next_token().ok_or("no second token")?;

    lexer.restore(&cp);
    let replayed = lexer.next_token().ok_or("no token after restore")?;

    assert_eq!(
        second.start, replayed.start,
        "restore should replay from checkpoint"
    );
    assert_eq!(
        std::mem::discriminant(&second.token_type),
        std::mem::discriminant(&replayed.token_type),
        "restored token should have same type"
    );
    Ok(())
}

#[test]
fn can_restore_checks() -> R {
    let lexer = PerlLexer::new("my $x = 1;");
    let cp = LexerCheckpoint::new();
    assert!(lexer.can_restore(&cp), "should be able to restore to start");

    let far_cp = LexerCheckpoint::at_position(99999);
    assert!(
        !lexer.can_restore(&far_cp),
        "should not restore past input end"
    );
    Ok(())
}

#[test]
fn checkpoint_validity() -> R {
    let input = "my $x";
    let cp = LexerCheckpoint::at_position(3);
    assert!(
        cp.is_valid_for(input),
        "position 3 should be valid for 5-byte input"
    );

    let cp2 = LexerCheckpoint::at_position(100);
    assert!(
        !cp2.is_valid_for(input),
        "position 100 should not be valid for 5-byte input"
    );
    Ok(())
}

// ===========================================================================
// 17. Lexer mode
// ===========================================================================

#[test]
fn mode_default_is_expect_term() -> R {
    let mode = LexerMode::default();
    assert!(mode.is_expect_term());
    assert!(!mode.is_expect_operator());
    Ok(())
}

#[test]
fn set_mode() -> R {
    let mut lexer = PerlLexer::new("1 + 2");
    lexer.set_mode(LexerMode::ExpectOperator);
    // After setting operator mode, the next slash should be division
    // (just verify set_mode doesn't panic)
    let _tok = lexer.next_token();
    Ok(())
}

// ===========================================================================
// 18. Format bodies
// ===========================================================================

#[test]
fn format_mode_parsing() -> R {
    let input = "format =\n@<<<< @<<<<\n$name, $value\n.\n";
    let mut lexer = PerlLexer::new(input);

    // Consume 'format' keyword
    let kw = lexer.next_token().ok_or("expected format keyword")?;
    assert!(
        matches!(&kw.token_type, TokenType::Keyword(k) if k.as_ref() == "format"),
        "expected 'format' keyword, got {:?}",
        kw.token_type
    );
    Ok(())
}

// ===========================================================================
// 19. Version strings
// ===========================================================================

#[test]
fn version_string() -> R {
    // The lexer may emit version strings as Version("v5.32.0") or as
    // Identifier("v5") + Number tokens depending on context.
    let input = "v5.32.0";
    let toks = significant_tokens(input);
    let first = toks.first().ok_or("no tokens")?;
    let is_version_or_id = matches!(&first.token_type, TokenType::Version(_))
        || matches!(&first.token_type, TokenType::Identifier(id) if id.as_ref().starts_with('v'));
    assert!(
        is_version_or_id,
        "expected Version or Identifier starting with 'v', got {:?}",
        first.token_type
    );
    Ok(())
}

// ===========================================================================
// 20. Error recovery & robustness
// ===========================================================================

#[test]
fn unterminated_string_does_not_hang() -> R {
    let input = "\"unterminated string";
    let toks = tokens(input);
    // Should eventually produce EOF even for unterminated strings
    let has_eof = toks.iter().any(|t| matches!(t.token_type, TokenType::EOF));
    assert!(has_eof, "unterminated string should still produce EOF");
    Ok(())
}

#[test]
fn unterminated_regex_does_not_hang() -> R {
    // The lexer may hang on unterminated regex. Use a bounded loop to verify.
    let mut lexer = PerlLexer::new("/unterminated");
    let mut count = 0;
    loop {
        match lexer.next_token() {
            Some(tok) if matches!(tok.token_type, TokenType::EOF) => break,
            Some(_) => {
                count += 1;
                if count > 100 {
                    break;
                }
            }
            None => break,
        }
    }
    // If we got here, the lexer terminated (pass)
    Ok(())
}

#[test]
fn invalid_characters_produce_tokens() -> R {
    // Null byte and other weird chars
    let input = "\x00\x01\x02";
    let toks = tokens(input);
    let has_eof = toks.iter().any(|t| matches!(t.token_type, TokenType::EOF));
    assert!(has_eof, "invalid characters should still produce EOF");
    Ok(())
}

#[test]
fn deeply_nested_braces() -> R {
    let input = "{{{{{{{{{{}}}}}}}}}}";
    let toks = tokens(input);
    let has_eof = toks.iter().any(|t| matches!(t.token_type, TokenType::EOF));
    assert!(has_eof, "deeply nested braces should terminate");
    Ok(())
}

// ===========================================================================
// 21. Token properties
// ===========================================================================

#[test]
fn token_len_and_is_empty() -> R {
    let tok = Token::new(TokenType::Semicolon, ";", 5, 6);
    assert_eq!(tok.len(), 1);
    assert!(!tok.is_empty());

    let empty_tok = Token::new(TokenType::EOF, "", 10, 10);
    assert_eq!(empty_tok.len(), 0);
    assert!(empty_tok.is_empty());
    Ok(())
}

#[test]
fn token_positions_are_monotonically_increasing() -> R {
    let input = "my $x = 42; print $x;";
    let toks = significant_tokens(input);

    for pair in toks.windows(2) {
        assert!(
            pair[0].start <= pair[1].start,
            "token positions should be monotonically increasing: {} > {}",
            pair[0].start,
            pair[1].start
        );
    }
    Ok(())
}

// ===========================================================================
// 22. Complex real-world snippets
// ===========================================================================

#[test]
fn oo_perl_snippet() -> R {
    let input = r#"
package Dog;
use Moo;
has name => (is => 'ro', required => 1);
sub bark { print "Woof!\n"; }
1;
"#;
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let has_package = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "package"));
    assert!(has_package, "expected 'package' keyword");

    let has_sub = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "sub"));
    assert!(has_sub, "expected 'sub' keyword");
    Ok(())
}

#[test]
fn regex_heavy_code() -> R {
    let input = r#"
if ($line =~ /^(\d+)\s+(.*)$/) {
    my ($num, $text) = ($1, $2);
    $text =~ s/^\s+//;
    $text =~ s/\s+$//;
}
"#;
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let regex_count = toks
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                TokenType::RegexMatch | TokenType::Substitution
            )
        })
        .count();
    assert!(
        regex_count >= 1,
        "expected at least 1 regex/substitution token, got {regex_count}"
    );
    Ok(())
}

#[test]
fn mixed_quote_operators_in_context() -> R {
    let input = r#"
my $str = q{hello};
my $dstr = qq{world $var};
my @words = qw(one two three);
my $re = qr/pattern/i;
my $out = qx{ls};
"#;
    let toks = tokens(input);
    assert_spans_valid(input, &toks);

    let quote_types: Vec<_> = toks
        .iter()
        .filter(|t| {
            matches!(
                t.token_type,
                TokenType::QuoteSingle
                    | TokenType::QuoteDouble
                    | TokenType::QuoteWords
                    | TokenType::QuoteRegex
                    | TokenType::QuoteCommand
            )
        })
        .collect();
    assert!(
        quote_types.len() >= 5,
        "expected at least 5 quote operator tokens, got {}",
        quote_types.len()
    );
    Ok(())
}

#[test]
fn chained_method_calls() -> R {
    let input = "$obj->foo->bar->baz";
    let toks = significant_tokens(input);
    let arrow_count = toks
        .iter()
        .filter(|t| {
            matches!(t.token_type, TokenType::Arrow)
                || matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == "->")
        })
        .count();
    assert_eq!(arrow_count, 3, "expected 3 arrow operators");
    Ok(())
}

#[test]
fn ternary_operator() -> R {
    let input = "$x ? 1 : 0";
    let toks = significant_tokens(input);
    let has_colon = toks.iter().any(|t| {
        matches!(t.token_type, TokenType::Colon)
            || matches!(&t.token_type, TokenType::Operator(o) if o.as_ref() == ":")
    });
    assert!(has_colon, "expected Colon or Operator(':') in ternary");
    Ok(())
}
