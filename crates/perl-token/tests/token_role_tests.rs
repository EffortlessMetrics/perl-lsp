use perl_token::TokenKind;

#[test]
fn assignment_operator_roles_match_expected_kinds() {
    let yes = [
        TokenKind::Assign,
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
    ];
    for kind in yes {
        assert!(kind.is_assignment_operator(), "{kind:?} should be assignment");
    }
    for kind in [TokenKind::Equal, TokenKind::WordAnd, TokenKind::Identifier] {
        assert!(!kind.is_assignment_operator(), "{kind:?} should not be assignment");
    }
}

#[test]
fn comparison_operator_roles_match_expected_kinds() {
    let yes = [
        TokenKind::Equal,
        TokenKind::NotEqual,
        TokenKind::Less,
        TokenKind::Greater,
        TokenKind::LessEqual,
        TokenKind::GreaterEqual,
        TokenKind::Spaceship,
        TokenKind::StringCompare,
        TokenKind::Match,
        TokenKind::NotMatch,
        TokenKind::SmartMatch,
    ];
    for kind in yes {
        assert!(kind.is_comparison_operator(), "{kind:?} should be comparison");
    }
    for kind in [TokenKind::Assign, TokenKind::WordOr, TokenKind::Identifier] {
        assert!(!kind.is_comparison_operator(), "{kind:?} should not be comparison");
    }
}

#[test]
fn logical_and_word_operator_roles_match_expected_kinds() {
    for kind in [TokenKind::And, TokenKind::Or, TokenKind::Not, TokenKind::DefinedOr] {
        assert!(kind.is_logical_operator(), "{kind:?} should be logical");
    }
    for kind in [TokenKind::WordAnd, TokenKind::WordOr, TokenKind::WordNot, TokenKind::WordXor] {
        assert!(kind.is_word_operator(), "{kind:?} should be word operator");
        assert!(
            kind.is_low_precedence_word_operator(),
            "{kind:?} should be low-precedence word operator"
        );
    }

    for kind in [TokenKind::Identifier, TokenKind::StringCompare, TokenKind::Assign] {
        assert!(!kind.is_logical_operator(), "{kind:?} should not be logical");
        assert!(!kind.is_word_operator(), "{kind:?} should not be word operator");
    }
}

#[test]
fn delimiter_roles_and_matching_are_symmetric() {
    for (open, close) in [
        (TokenKind::LeftParen, TokenKind::RightParen),
        (TokenKind::LeftBrace, TokenKind::RightBrace),
        (TokenKind::LeftBracket, TokenKind::RightBracket),
    ] {
        assert!(open.is_open_delimiter());
        assert!(close.is_close_delimiter());
        assert_eq!(open.matching_delimiter(), Some(close));
        assert_eq!(close.matching_delimiter(), Some(open));
    }

    for kind in [TokenKind::Comma, TokenKind::Semicolon, TokenKind::Identifier] {
        assert!(!kind.is_open_delimiter());
        assert!(!kind.is_close_delimiter());
        assert_eq!(kind.matching_delimiter(), None);
    }
}

#[test]
fn quote_like_role_matches_expected_kinds() {
    for kind in [
        TokenKind::Regex,
        TokenKind::Substitution,
        TokenKind::Transliteration,
        TokenKind::QuoteSingle,
        TokenKind::QuoteDouble,
        TokenKind::QuoteWords,
        TokenKind::QuoteCommand,
        TokenKind::HeredocStart,
    ] {
        assert!(kind.is_quote_like(), "{kind:?} should be quote-like");
    }

    for kind in [TokenKind::String, TokenKind::Identifier, TokenKind::HeredocBody] {
        assert!(!kind.is_quote_like(), "{kind:?} should not be quote-like");
    }
}

#[test]
fn recovery_boundary_role_matches_expected_kinds() {
    for kind in [
        TokenKind::Semicolon,
        TokenKind::Eof,
        TokenKind::RightParen,
        TokenKind::RightBrace,
        TokenKind::RightBracket,
        TokenKind::DataMarker,
        TokenKind::My,
        TokenKind::Our,
        TokenKind::Local,
        TokenKind::State,
        TokenKind::Sub,
        TokenKind::Package,
        TokenKind::Use,
        TokenKind::No,
        TokenKind::If,
        TokenKind::Unless,
        TokenKind::Elsif,
        TokenKind::Else,
        TokenKind::While,
        TokenKind::Until,
        TokenKind::For,
        TokenKind::Foreach,
    ] {
        assert!(kind.is_recovery_boundary(), "{kind:?} should be recovery boundary");
    }

    for kind in [TokenKind::LeftBrace, TokenKind::Comma, TokenKind::Identifier] {
        assert!(!kind.is_recovery_boundary(), "{kind:?} should not be recovery boundary");
    }
}
