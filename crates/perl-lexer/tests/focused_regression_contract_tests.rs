//! Focused lexer regressions for ambiguity, quote-like ops, transliteration,
//! heredoc edge-cases, unicode/BOM, v-strings, and termination contracts.

use perl_lexer::{PerlLexer, Token, TokenType};

type R = Result<(), Box<dyn std::error::Error>>;

fn tokens(input: &str) -> Vec<Token> {
    PerlLexer::new(input).collect_tokens()
}

fn non_trivia(input: &str) -> Vec<Token> {
    tokens(input)
        .into_iter()
        .filter(|t| !matches!(t.token_type, TokenType::Whitespace | TokenType::Newline | TokenType::EOF))
        .collect()
}

fn assert_valid_spans(input: &str, toks: &[Token]) {
    for tok in toks {
        assert!(tok.start <= tok.end, "invalid span ordering for {:?}", tok.token_type);
        assert!(tok.end <= input.len(), "token end out of bounds for {:?}", tok.token_type);
        assert_eq!(&input[tok.start..tok.end], tok.text.as_ref(), "token text/span mismatch for {:?}", tok.token_type);
    }
}

#[test]
fn slash_division_after_quote_double_literal() -> R {
    let input = "qq{abc} / 2;";
    let toks = tokens(input);
    assert_valid_spans(input, &toks);

    let slash = toks
        .iter()
        .find(|t| matches!(t.token_type, TokenType::Division))
        .ok_or("expected Division token")?;
    assert_eq!(slash.text.as_ref(), "/");
    Ok(())
}

#[test]
fn slash_regex_after_binding_operator() -> R {
    let input = "$x =~ /abc/i;";
    let toks = non_trivia(input);
    let regex = toks
        .iter()
        .find(|t| matches!(t.token_type, TokenType::RegexMatch))
        .ok_or("expected RegexMatch token")?;
    assert_eq!(regex.text.as_ref(), "/abc/i");
    Ok(())
}

#[test]
fn quote_words_angle_delimiter_token_contract() -> R {
    let input = "qw<alpha beta gamma>";
    let toks = non_trivia(input);
    let first = toks.first().ok_or("expected at least one token")?;
    assert!(matches!(first.token_type, TokenType::QuoteWords));
    assert_eq!(first.text.as_ref(), input);
    Ok(())
}

#[test]
fn quote_command_backticks_token_contract() -> R {
    let input = "`uname -a`";
    let toks = non_trivia(input);
    let first = toks.first().ok_or("expected at least one token")?;
    assert!(matches!(first.token_type, TokenType::QuoteCommand));
    assert_eq!(first.text.as_ref(), input);
    Ok(())
}

#[test]
fn transliteration_with_modifiers_token_contract() -> R {
    let input = "tr/a-z/A-Z/cds";
    let toks = non_trivia(input);
    let tr = toks
        .iter()
        .find(|t| matches!(t.token_type, TokenType::Transliteration))
        .ok_or("expected Transliteration token")?;
    assert_eq!(tr.text.as_ref(), input);
    Ok(())
}

#[test]
fn incomplete_transliteration_is_single_token_and_terminates() -> R {
    let input = "tr/a-z";
    let toks = non_trivia(input);
    let tr = toks
        .iter()
        .find(|t| matches!(t.token_type, TokenType::Transliteration))
        .ok_or("expected Transliteration token")?;
    assert_eq!(tr.text.as_ref(), input);

    let all = tokens(input);
    assert!(all.iter().any(|t| matches!(t.token_type, TokenType::EOF)), "expected EOF token");
    Ok(())
}

#[test]
fn heredoc_crlf_terminates_and_allows_following_statement() -> R {
    let input = "<<EOF\r\nbody\r\nEOF\r\nmy $x;\r\n";
    let toks = non_trivia(input);
    let heredoc = toks
        .iter()
        .find(|t| matches!(t.token_type, TokenType::HeredocStart))
        .ok_or("expected HeredocStart token")?;
    assert_eq!(heredoc.text.as_ref(), "<<EOF");

    let has_my_keyword = toks
        .iter()
        .any(|t| matches!(&t.token_type, TokenType::Keyword(k) if k.as_ref() == "my"));
    assert!(has_my_keyword, "expected 'my' keyword after heredoc terminator");
    Ok(())
}

#[test]
fn heredoc_unterminated_still_terminates() -> R {
    let input = "<<EOF\nbody\n";
    let toks = tokens(input);
    assert_valid_spans(input, &toks);
    assert!(toks.iter().any(|t| matches!(t.token_type, TokenType::EOF)), "expected EOF token");
    Ok(())
}

#[test]
fn bom_skipped_before_first_keyword_token() -> R {
    let input = "\u{FEFF}my $x = 1;\n";
    let toks = non_trivia(input);
    let first = toks.first().ok_or("expected at least one token")?;
    assert!(matches!(&first.token_type, TokenType::Keyword(k) if k.as_ref() == "my"));
    assert_eq!(first.text.as_ref(), "my");
    assert_eq!(first.start, '\u{FEFF}'.len_utf8(), "first token should start after UTF-8 BOM");
    Ok(())
}

#[test]
fn unicode_prefix_near_heredoc_has_valid_spans_and_termination() -> R {
    let input = "¡<<'END'\ntext\nEND\n";
    let toks = tokens(input);
    assert_valid_spans(input, &toks);
    assert!(toks.iter().any(|t| matches!(t.token_type, TokenType::EOF)), "expected EOF token");
    Ok(())
}

#[test]
fn vstring_text_and_span_contract() -> R {
    let input = "use v5.38.2;\n";
    let toks = non_trivia(input);
    let version = toks
        .iter()
        .find(|t| matches!(t.token_type, TokenType::Version(_)))
        .ok_or("expected Version token")?;
    assert_eq!(version.text.as_ref(), "v5.38.2");
    assert_eq!(&input[version.start..version.end], "v5.38.2");
    Ok(())
}
