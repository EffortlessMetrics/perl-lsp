use perl_lexer::{Token as LexerToken, TokenType as LexerTokenType};
use perl_parser_core::{TokenKind, TokenStream};
use perl_tdd_support::must;

#[test]
fn token_stream_conversion_maps_sigils_unknown_and_unknown_rest()
-> Result<(), Box<dyn std::error::Error>> {
    let raw = vec![
        LexerToken::new(LexerTokenType::Identifier("$".into()), "$", 0, 1),
        LexerToken::new(LexerTokenType::Identifier("@".into()), "@", 1, 2),
        LexerToken::new(LexerTokenType::Identifier("%".into()), "%", 2, 3),
        LexerToken::new(LexerTokenType::Identifier("&".into()), "&", 3, 4),
        LexerToken::new(LexerTokenType::Error("bad".into()), "§", 4, 5),
        LexerToken::new(LexerTokenType::UnknownRest, "...", 5, 8),
    ];

    let converted = TokenStream::lexer_tokens_to_parser_tokens(raw)
        .into_iter()
        .map(|t| t.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        converted,
        vec![
            TokenKind::ScalarSigil,
            TokenKind::ArraySigil,
            TokenKind::HashSigil,
            TokenKind::SubSigil,
            TokenKind::Unknown,
            TokenKind::UnknownRest,
        ]
    );

    Ok(())
}

#[test]
fn token_stream_conformance_tables_cover_required_token_kinds()
-> Result<(), Box<dyn std::error::Error>> {
    let covered = vec![
        TokenKind::My,
        TokenKind::Our,
        TokenKind::Local,
        TokenKind::State,
        TokenKind::Sub,
        TokenKind::If,
        TokenKind::Elsif,
        TokenKind::Else,
        TokenKind::Unless,
        TokenKind::While,
        TokenKind::Until,
        TokenKind::For,
        TokenKind::Foreach,
        TokenKind::Return,
        TokenKind::Package,
        TokenKind::Use,
        TokenKind::No,
        TokenKind::Begin,
        TokenKind::End,
        TokenKind::Check,
        TokenKind::Init,
        TokenKind::Unitcheck,
        TokenKind::Eval,
        TokenKind::Do,
        TokenKind::Given,
        TokenKind::When,
        TokenKind::Default,
        TokenKind::Try,
        TokenKind::Catch,
        TokenKind::Finally,
        TokenKind::Continue,
        TokenKind::Next,
        TokenKind::Last,
        TokenKind::Redo,
        TokenKind::Goto,
        TokenKind::Class,
        TokenKind::Method,
        TokenKind::Field,
        TokenKind::Format,
        TokenKind::Undef,
        TokenKind::Defer,
        TokenKind::Assign,
        TokenKind::Plus,
        TokenKind::Minus,
        TokenKind::Star,
        TokenKind::Slash,
        TokenKind::Percent,
        TokenKind::Power,
        TokenKind::LeftShift,
        TokenKind::RightShift,
        TokenKind::BitwiseAnd,
        TokenKind::BitwiseOr,
        TokenKind::BitwiseXor,
        TokenKind::BitwiseNot,
        TokenKind::PlusAssign,
        TokenKind::MinusAssign,
        TokenKind::StarAssign,
        TokenKind::SlashAssign,
        TokenKind::PercentAssign,
        TokenKind::DotAssign,
        TokenKind::AndAssign,
        TokenKind::OrAssign,
        TokenKind::XorAssign,
        TokenKind::PowerAssign,
        TokenKind::LeftShiftAssign,
        TokenKind::RightShiftAssign,
        TokenKind::LogicalAndAssign,
        TokenKind::LogicalOrAssign,
        TokenKind::DefinedOrAssign,
        TokenKind::Equal,
        TokenKind::NotEqual,
        TokenKind::Match,
        TokenKind::NotMatch,
        TokenKind::SmartMatch,
        TokenKind::Less,
        TokenKind::Greater,
        TokenKind::LessEqual,
        TokenKind::GreaterEqual,
        TokenKind::Spaceship,
        TokenKind::StringCompare,
        TokenKind::And,
        TokenKind::Or,
        TokenKind::Not,
        TokenKind::DefinedOr,
        TokenKind::WordAnd,
        TokenKind::WordOr,
        TokenKind::WordNot,
        TokenKind::WordXor,
        TokenKind::Arrow,
        TokenKind::FatArrow,
        TokenKind::Dot,
        TokenKind::Range,
        TokenKind::Ellipsis,
        TokenKind::Increment,
        TokenKind::Decrement,
        TokenKind::DoubleColon,
        TokenKind::Question,
        TokenKind::Colon,
        TokenKind::Backslash,
        TokenKind::LeftParen,
        TokenKind::RightParen,
        TokenKind::LeftBrace,
        TokenKind::RightBrace,
        TokenKind::LeftBracket,
        TokenKind::RightBracket,
        TokenKind::Semicolon,
        TokenKind::Comma,
        TokenKind::ScalarSigil,
        TokenKind::ArraySigil,
        TokenKind::HashSigil,
        TokenKind::SubSigil,
        TokenKind::QuoteSingle,
        TokenKind::QuoteDouble,
        TokenKind::QuoteWords,
        TokenKind::QuoteCommand,
        TokenKind::Regex,
        TokenKind::Substitution,
        TokenKind::Transliteration,
        TokenKind::HeredocStart,
        TokenKind::HeredocBody,
        TokenKind::DataMarker,
        TokenKind::DataBody,
        TokenKind::Unknown,
        TokenKind::UnknownRest,
        TokenKind::Eof,
    ];

    let required = TokenKind::ALL
        .iter()
        .copied()
        .filter(|kind| {
            matches!(
                kind,
                TokenKind::My
                    | TokenKind::Our
                    | TokenKind::Local
                    | TokenKind::State
                    | TokenKind::Sub
                    | TokenKind::If
                    | TokenKind::Elsif
                    | TokenKind::Else
                    | TokenKind::Unless
                    | TokenKind::While
                    | TokenKind::Until
                    | TokenKind::For
                    | TokenKind::Foreach
                    | TokenKind::Return
                    | TokenKind::Package
                    | TokenKind::Use
                    | TokenKind::No
                    | TokenKind::Begin
                    | TokenKind::End
                    | TokenKind::Check
                    | TokenKind::Init
                    | TokenKind::Unitcheck
                    | TokenKind::Eval
                    | TokenKind::Do
                    | TokenKind::Given
                    | TokenKind::When
                    | TokenKind::Default
                    | TokenKind::Try
                    | TokenKind::Catch
                    | TokenKind::Finally
                    | TokenKind::Continue
                    | TokenKind::Next
                    | TokenKind::Last
                    | TokenKind::Redo
                    | TokenKind::Goto
                    | TokenKind::Class
                    | TokenKind::Method
                    | TokenKind::Field
                    | TokenKind::Format
                    | TokenKind::Undef
                    | TokenKind::Defer
                    | TokenKind::Assign
                    | TokenKind::Plus
                    | TokenKind::Minus
                    | TokenKind::Star
                    | TokenKind::Slash
                    | TokenKind::Percent
                    | TokenKind::Power
                    | TokenKind::LeftShift
                    | TokenKind::RightShift
                    | TokenKind::BitwiseAnd
                    | TokenKind::BitwiseOr
                    | TokenKind::BitwiseXor
                    | TokenKind::BitwiseNot
                    | TokenKind::PlusAssign
                    | TokenKind::MinusAssign
                    | TokenKind::StarAssign
                    | TokenKind::SlashAssign
                    | TokenKind::PercentAssign
                    | TokenKind::DotAssign
                    | TokenKind::AndAssign
                    | TokenKind::OrAssign
                    | TokenKind::XorAssign
                    | TokenKind::PowerAssign
                    | TokenKind::LeftShiftAssign
                    | TokenKind::RightShiftAssign
                    | TokenKind::LogicalAndAssign
                    | TokenKind::LogicalOrAssign
                    | TokenKind::DefinedOrAssign
                    | TokenKind::Equal
                    | TokenKind::NotEqual
                    | TokenKind::Match
                    | TokenKind::NotMatch
                    | TokenKind::SmartMatch
                    | TokenKind::Less
                    | TokenKind::Greater
                    | TokenKind::LessEqual
                    | TokenKind::GreaterEqual
                    | TokenKind::Spaceship
                    | TokenKind::StringCompare
                    | TokenKind::And
                    | TokenKind::Or
                    | TokenKind::Not
                    | TokenKind::DefinedOr
                    | TokenKind::WordAnd
                    | TokenKind::WordOr
                    | TokenKind::WordNot
                    | TokenKind::WordXor
                    | TokenKind::Arrow
                    | TokenKind::FatArrow
                    | TokenKind::Dot
                    | TokenKind::Range
                    | TokenKind::Ellipsis
                    | TokenKind::Increment
                    | TokenKind::Decrement
                    | TokenKind::DoubleColon
                    | TokenKind::Question
                    | TokenKind::Colon
                    | TokenKind::Backslash
                    | TokenKind::LeftParen
                    | TokenKind::RightParen
                    | TokenKind::LeftBrace
                    | TokenKind::RightBrace
                    | TokenKind::LeftBracket
                    | TokenKind::RightBracket
                    | TokenKind::Semicolon
                    | TokenKind::Comma
                    | TokenKind::ScalarSigil
                    | TokenKind::ArraySigil
                    | TokenKind::HashSigil
                    | TokenKind::SubSigil
                    | TokenKind::QuoteSingle
                    | TokenKind::QuoteDouble
                    | TokenKind::QuoteWords
                    | TokenKind::QuoteCommand
                    | TokenKind::Regex
                    | TokenKind::Substitution
                    | TokenKind::Transliteration
                    | TokenKind::HeredocStart
                    | TokenKind::HeredocBody
                    | TokenKind::DataMarker
                    | TokenKind::DataBody
                    | TokenKind::Unknown
                    | TokenKind::UnknownRest
                    | TokenKind::Eof
            )
        })
        .collect::<Vec<_>>();

    for kind in &required {
        assert!(
            covered.contains(kind),
            "required TokenKind missing from conformance tables: {kind:?}"
        );
    }
    assert_eq!(covered.len(), required.len(), "conformance table size drift");
    Ok(())
}

#[test]
fn token_stream_eof_is_produced_by_token_stream() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TokenStream::new("my $x");
    loop {
        let token = must(stream.next());
        if token.kind == TokenKind::Eof {
            break;
        }
    }

    assert_eq!(must(stream.next()).kind, TokenKind::Eof);
    Ok(())
}
