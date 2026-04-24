//! Tests for TokenKind metadata table and display behavior.

use perl_token::{TokenCategory, TokenKind};

#[test]
fn display_name_delegates_to_info() {
    for kind in TokenKind::all() {
        let info = kind.info();
        assert_eq!(kind.display_name(), info.display_name, "display_name mismatch for {kind:?}");
    }
}

#[test]
fn every_variant_is_in_the_metadata_table() {
    let kinds = TokenKind::all();
    assert!(!kinds.is_empty());

    // If a new variant is added and omitted from metadata, this match becomes
    // non-exhaustive and fails to compile.
    for kind in kinds {
        match kind {
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
            | TokenKind::Number
            | TokenKind::String
            | TokenKind::Regex
            | TokenKind::Substitution
            | TokenKind::Transliteration
            | TokenKind::QuoteSingle
            | TokenKind::QuoteDouble
            | TokenKind::QuoteWords
            | TokenKind::QuoteCommand
            | TokenKind::HeredocStart
            | TokenKind::HeredocBody
            | TokenKind::FormatBody
            | TokenKind::DataMarker
            | TokenKind::DataBody
            | TokenKind::VString
            | TokenKind::UnknownRest
            | TokenKind::HeredocDepthLimit
            | TokenKind::Identifier
            | TokenKind::ScalarSigil
            | TokenKind::ArraySigil
            | TokenKind::HashSigil
            | TokenKind::SubSigil
            | TokenKind::GlobSigil
            | TokenKind::Eof
            | TokenKind::Unknown => {}
        }
    }
}

#[test]
fn categories_are_mutually_exclusive_and_predicates_match() {
    for kind in TokenKind::all() {
        let predicates = [
            kind.is_keyword(),
            kind.is_operator(),
            kind.is_delimiter(),
            kind.is_literal(),
            kind.is_sigil(),
            kind.is_special(),
            matches!(kind.category(), TokenCategory::Identifier),
        ];

        let active = predicates.iter().filter(|&&v| v).count();
        assert_eq!(active, 1, "{kind:?} should have exactly one category");

        assert_eq!(kind.is_keyword(), matches!(kind.category(), TokenCategory::Keyword));
        assert_eq!(kind.is_operator(), matches!(kind.category(), TokenCategory::Operator));
        assert_eq!(kind.is_delimiter(), matches!(kind.category(), TokenCategory::Delimiter));
        assert_eq!(kind.is_literal(), matches!(kind.category(), TokenCategory::Literal));
        assert_eq!(kind.is_sigil(), matches!(kind.category(), TokenCategory::Sigil));
        assert_eq!(
            kind.is_identifier_like(),
            matches!(kind.category(), TokenCategory::Identifier | TokenCategory::Sigil),
        );
        assert_eq!(kind.is_special(), matches!(kind.category(), TokenCategory::Special));
    }
}

#[test]
fn keyword_and_operator_spelling_align_with_category() {
    for kind in TokenKind::all() {
        let info = kind.info();

        match info.category {
            TokenCategory::Keyword => {
                assert!(info.keyword_spelling.is_some(), "{kind:?} should expose keyword_spelling");
                assert!(
                    info.operator_spelling.is_none(),
                    "{kind:?} should not expose operator_spelling"
                );
            }
            TokenCategory::Operator => {
                assert!(
                    info.operator_spelling.is_some(),
                    "{kind:?} should expose operator_spelling"
                );
                assert!(
                    info.keyword_spelling.is_none(),
                    "{kind:?} should not expose keyword_spelling"
                );
            }
            _ => {
                assert!(
                    info.keyword_spelling.is_none(),
                    "{kind:?} should not expose keyword_spelling"
                );
                assert!(
                    info.operator_spelling.is_none(),
                    "{kind:?} should not expose operator_spelling"
                );
            }
        }
    }
}
