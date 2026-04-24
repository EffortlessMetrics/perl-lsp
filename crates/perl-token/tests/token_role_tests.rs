use perl_token::TokenKind;

#[test]
fn assignment_operator_role_is_precise() {
    let assignment_ops = [
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
    for kind in assignment_ops {
        assert!(kind.is_assignment_operator(), "{kind:?} should be assignment");
    }

    for kind in [TokenKind::Plus, TokenKind::Equal, TokenKind::WordAnd, TokenKind::Identifier] {
        assert!(!kind.is_assignment_operator(), "{kind:?} must not be assignment");
    }
}

#[test]
fn comparison_operator_role_is_precise() {
    let comparison_ops = [
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
    ];
    for kind in comparison_ops {
        assert!(kind.is_comparison_operator(), "{kind:?} should be comparison");
    }
    for kind in [TokenKind::Assign, TokenKind::And, TokenKind::WordOr] {
        assert!(!kind.is_comparison_operator(), "{kind:?} must not be comparison");
    }
}

#[test]
fn logical_and_word_operator_roles_are_precise() {
    let logical_ops = [
        TokenKind::And,
        TokenKind::Or,
        TokenKind::Not,
        TokenKind::DefinedOr,
        TokenKind::WordAnd,
        TokenKind::WordOr,
        TokenKind::WordNot,
        TokenKind::WordXor,
    ];
    for kind in logical_ops {
        assert!(kind.is_logical_operator(), "{kind:?} should be logical");
    }
    for kind in [TokenKind::Plus, TokenKind::StringCompare, TokenKind::Identifier] {
        assert!(!kind.is_logical_operator(), "{kind:?} must not be logical");
    }

    let word_ops = [
        TokenKind::WordAnd,
        TokenKind::WordOr,
        TokenKind::WordNot,
        TokenKind::WordXor,
        TokenKind::StringCompare,
    ];
    for kind in word_ops {
        assert!(kind.is_word_operator(), "{kind:?} should be word operator");
    }
    for kind in [TokenKind::And, TokenKind::Or, TokenKind::Not] {
        assert!(!kind.is_word_operator(), "{kind:?} must not be word operator");
    }

    let low_precedence_word_ops = [
        TokenKind::WordAnd,
        TokenKind::WordOr,
        TokenKind::WordNot,
        TokenKind::WordXor,
    ];
    for kind in low_precedence_word_ops {
        assert!(
            kind.is_low_precedence_word_operator(),
            "{kind:?} should be low precedence word op"
        );
    }
    assert!(!TokenKind::StringCompare.is_low_precedence_word_operator());
}

#[test]
fn delimiter_roles_and_matching_are_precise() {
    for open in [TokenKind::LeftParen, TokenKind::LeftBrace, TokenKind::LeftBracket] {
        assert!(open.is_open_delimiter(), "{open:?} should open delimiter");
        assert!(!open.is_close_delimiter(), "{open:?} should not close delimiter");
        assert!(open.matching_delimiter().is_some(), "{open:?} should have match");
    }

    for close in [TokenKind::RightParen, TokenKind::RightBrace, TokenKind::RightBracket] {
        assert!(close.is_close_delimiter(), "{close:?} should close delimiter");
        assert!(!close.is_open_delimiter(), "{close:?} should not open delimiter");
        assert!(close.matching_delimiter().is_some(), "{close:?} should have match");
    }

    assert_eq!(
        TokenKind::LeftParen.matching_delimiter(),
        Some(TokenKind::RightParen)
    );
    assert_eq!(
        TokenKind::RightParen.matching_delimiter(),
        Some(TokenKind::LeftParen)
    );
    assert_eq!(
        TokenKind::LeftBrace.matching_delimiter(),
        Some(TokenKind::RightBrace)
    );
    assert_eq!(
        TokenKind::RightBrace.matching_delimiter(),
        Some(TokenKind::LeftBrace)
    );
    assert_eq!(
        TokenKind::LeftBracket.matching_delimiter(),
        Some(TokenKind::RightBracket)
    );
    assert_eq!(
        TokenKind::RightBracket.matching_delimiter(),
        Some(TokenKind::LeftBracket)
    );
    assert_eq!(TokenKind::Identifier.matching_delimiter(), None);
}

#[test]
fn quote_like_and_recovery_boundary_roles_are_precise() {
    let quote_like = [
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
    ];
    for kind in quote_like {
        assert!(kind.is_quote_like(), "{kind:?} should be quote-like");
    }
    for kind in [TokenKind::Identifier, TokenKind::Number, TokenKind::FormatBody] {
        assert!(!kind.is_quote_like(), "{kind:?} must not be quote-like");
    }

    let recovery_boundaries = [
        TokenKind::Semicolon,
        TokenKind::RightParen,
        TokenKind::RightBrace,
        TokenKind::RightBracket,
        TokenKind::Eof,
    ];
    for kind in recovery_boundaries {
        assert!(kind.is_recovery_boundary(), "{kind:?} should be recovery boundary");
    }
    for kind in [TokenKind::LeftParen, TokenKind::Identifier, TokenKind::Comma] {
        assert!(!kind.is_recovery_boundary(), "{kind:?} must not be recovery boundary");
    }
}
