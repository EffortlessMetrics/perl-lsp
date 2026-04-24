use perl_token::TokenKind;

fn all_kinds() -> Vec<TokenKind> {
    vec![
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
        TokenKind::Number,
        TokenKind::String,
        TokenKind::Regex,
        TokenKind::Substitution,
        TokenKind::Transliteration,
        TokenKind::QuoteSingle,
        TokenKind::QuoteDouble,
        TokenKind::QuoteWords,
        TokenKind::QuoteCommand,
        TokenKind::HeredocStart,
        TokenKind::HeredocBody,
        TokenKind::FormatBody,
        TokenKind::DataMarker,
        TokenKind::DataBody,
        TokenKind::VString,
        TokenKind::UnknownRest,
        TokenKind::HeredocDepthLimit,
        TokenKind::Identifier,
        TokenKind::ScalarSigil,
        TokenKind::ArraySigil,
        TokenKind::HashSigil,
        TokenKind::SubSigil,
        TokenKind::GlobSigil,
        TokenKind::Eof,
        TokenKind::Unknown,
    ]
}

#[test]
fn token_role_predicates_are_exhaustive() {
    for kind in all_kinds() {
        assert_eq!(
            kind.is_assignment_operator(),
            matches!(
                kind,
                TokenKind::Assign
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
            ),
            "assignment mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_comparison_operator(),
            matches!(
                kind,
                TokenKind::Equal
                    | TokenKind::NotEqual
                    | TokenKind::Less
                    | TokenKind::Greater
                    | TokenKind::LessEqual
                    | TokenKind::GreaterEqual
                    | TokenKind::Spaceship
                    | TokenKind::StringCompare
                    | TokenKind::Match
                    | TokenKind::NotMatch
                    | TokenKind::SmartMatch
            ),
            "comparison mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_logical_operator(),
            matches!(kind, TokenKind::And | TokenKind::Or | TokenKind::DefinedOr | TokenKind::Not),
            "logical mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_word_operator(),
            matches!(
                kind,
                TokenKind::WordAnd | TokenKind::WordOr | TokenKind::WordNot | TokenKind::WordXor
            ),
            "word-op mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_low_precedence_word_operator(),
            matches!(
                kind,
                TokenKind::WordAnd | TokenKind::WordOr | TokenKind::WordNot | TokenKind::WordXor
            ),
            "low-precedence word-op mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_open_delimiter(),
            matches!(kind, TokenKind::LeftParen | TokenKind::LeftBrace | TokenKind::LeftBracket),
            "open-delimiter mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_close_delimiter(),
            matches!(kind, TokenKind::RightParen | TokenKind::RightBrace | TokenKind::RightBracket),
            "close-delimiter mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_quote_like(),
            matches!(
                kind,
                TokenKind::Regex
                    | TokenKind::Substitution
                    | TokenKind::Transliteration
                    | TokenKind::QuoteSingle
                    | TokenKind::QuoteDouble
                    | TokenKind::QuoteWords
                    | TokenKind::QuoteCommand
                    | TokenKind::HeredocStart
            ),
            "quote-like mismatch for {kind:?}"
        );

        assert_eq!(
            kind.is_recovery_boundary(),
            matches!(
                kind,
                TokenKind::Semicolon
                    | TokenKind::RightParen
                    | TokenKind::RightBrace
                    | TokenKind::RightBracket
                    | TokenKind::Eof
            ),
            "recovery-boundary mismatch for {kind:?}"
        );
    }
}

#[test]
fn matching_delimiter_maps_open_and_close_pairs() {
    assert_eq!(TokenKind::LeftParen.matching_delimiter(), Some(TokenKind::RightParen));
    assert_eq!(TokenKind::RightParen.matching_delimiter(), Some(TokenKind::LeftParen));
    assert_eq!(TokenKind::LeftBrace.matching_delimiter(), Some(TokenKind::RightBrace));
    assert_eq!(TokenKind::RightBrace.matching_delimiter(), Some(TokenKind::LeftBrace));
    assert_eq!(TokenKind::LeftBracket.matching_delimiter(), Some(TokenKind::RightBracket));
    assert_eq!(TokenKind::RightBracket.matching_delimiter(), Some(TokenKind::LeftBracket));

    assert_eq!(TokenKind::Semicolon.matching_delimiter(), None);
    assert_eq!(TokenKind::Identifier.matching_delimiter(), None);
}
