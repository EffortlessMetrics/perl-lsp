use std::error::Error;

use perl_lexer::{PerlLexer, TokenType};
use perl_parser_core::TokenStream;
use perl_token::{KEYWORD_SPELLINGS, TokenKind};

fn parser_kinds_for(input: &str) -> Vec<TokenKind> {
    let mut lexer = PerlLexer::new(input);
    let mut raw = Vec::new();

    while let Some(token) = lexer.next_token() {
        if matches!(token.token_type, TokenType::EOF) {
            break;
        }
        raw.push(token);
    }

    TokenStream::lexer_tokens_to_parser_tokens(raw).into_iter().map(|token| token.kind).collect()
}

#[test]
fn canonical_keyword_spellings_flow_from_lexer_to_parser_kind() -> Result<(), Box<dyn Error>> {
    for &(spelling, expected) in KEYWORD_SPELLINGS {
        let kinds = parser_kinds_for(spelling);
        assert_eq!(
            kinds.first().copied(),
            Some(expected),
            "keyword spelling {spelling:?} should produce {expected:?}, got {kinds:?}"
        );
    }

    Ok(())
}

#[test]
fn representative_operator_spellings_flow_from_lexer_to_parser_kind() -> Result<(), Box<dyn Error>>
{
    let cases = [
        ("$a = $b", TokenKind::Assign),
        ("$a += $b", TokenKind::PlusAssign),
        ("$a ** $b", TokenKind::Power),
        ("$a //= $b", TokenKind::DefinedOrAssign),
        ("$a == $b", TokenKind::Equal),
        ("$a =~ /foo/", TokenKind::Match),
        ("$a !~ /foo/", TokenKind::NotMatch),
        ("$a <=> $b", TokenKind::Spaceship),
        ("$a && $b", TokenKind::And),
        ("$a // $b", TokenKind::DefinedOr),
        ("$obj->method", TokenKind::Arrow),
        ("key => $value", TokenKind::FatArrow),
        ("1 .. 10", TokenKind::Range),
        ("1 ... 10", TokenKind::Ellipsis),
        ("$i++", TokenKind::Increment),
        ("$i--", TokenKind::Decrement),
        ("$a ? $b : $c", TokenKind::Question),
        ("$a ? $b : $c", TokenKind::Colon),
        ("\\@items", TokenKind::Backslash),
    ];

    for (source, expected) in cases {
        let kinds = parser_kinds_for(source);
        assert!(
            kinds.contains(&expected),
            "source {source:?} should contain {expected:?}, got {kinds:?}"
        );
    }

    Ok(())
}

#[test]
fn delimiters_and_live_sigil_tokens_keep_parser_kind_contract() -> Result<(), Box<dyn Error>> {
    let delimiter_kinds = parser_kinds_for("({[]}),;");
    assert_eq!(
        delimiter_kinds,
        vec![
            TokenKind::LeftParen,
            TokenKind::LeftBrace,
            TokenKind::LeftBracket,
            TokenKind::RightBracket,
            TokenKind::RightBrace,
            TokenKind::RightParen,
            TokenKind::Comma,
            TokenKind::Semicolon,
        ]
    );

    for (source, expected) in [("$", TokenKind::ScalarSigil), ("@", TokenKind::ArraySigil)] {
        let kinds = parser_kinds_for(source);
        assert_eq!(
            kinds.first().copied(),
            Some(expected),
            "source {source:?} should start with {expected:?}, got {kinds:?}"
        );
    }

    Ok(())
}

#[test]
fn quote_heredoc_and_data_tokens_keep_parser_specific_kinds() -> Result<(), Box<dyn Error>> {
    let quote_kinds = parser_kinds_for("my @words = qw(foo bar);");
    assert!(
        quote_kinds.contains(&TokenKind::QuoteWords),
        "qw() should flow as QuoteWords, got {quote_kinds:?}"
    );

    let heredoc_kinds = parser_kinds_for("my $text = <<'END';\nhello\nEND\n");
    assert!(
        heredoc_kinds.contains(&TokenKind::HeredocStart),
        "heredoc should include HeredocStart, got {heredoc_kinds:?}"
    );

    let data_kinds = parser_kinds_for("my $x = 1;\n__DATA__\npayload\n");
    assert!(
        data_kinds.contains(&TokenKind::DataMarker),
        "__DATA__ should include DataMarker, got {data_kinds:?}"
    );
    assert!(
        data_kinds.contains(&TokenKind::DataBody),
        "__DATA__ should include DataBody, got {data_kinds:?}"
    );

    Ok(())
}
